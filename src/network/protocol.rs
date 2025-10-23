use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::Result as IoResult;
use std::io::{Read, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RequestType {
    GetInstruments = 0,
    StartRecording = 1,
    StopRecording = 2,
    SpawnInstrument = 3,
    StopInstrument = 4,
    SendCommand = 5,
    Heartbeat = 6,
}

impl RequestType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(RequestType::GetInstruments),
            1 => Some(RequestType::StartRecording),
            2 => Some(RequestType::StopRecording),
            3 => Some(RequestType::SpawnInstrument),
            4 => Some(RequestType::StopInstrument),
            5 => Some(RequestType::SendCommand),
            6 => Some(RequestType::Heartbeat),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ResponseStatus {
    Success = 0,
    Error = 1,
    NotFound = 2,
    InvalidRequest = 3,
    Timeout = 4,
}

impl ResponseStatus {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(ResponseStatus::Success),
            1 => Some(ResponseStatus::Error),
            2 => Some(ResponseStatus::NotFound),
            3 => Some(ResponseStatus::InvalidRequest),
            4 => Some(ResponseStatus::Timeout),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlRequest {
    pub request_id: u32,
    pub request_type: RequestType,
    pub payload: Vec<u8>,
    pub timestamp: u64,
}

impl ControlRequest {
    pub fn new(request_id: u32, request_type: RequestType, payload: Vec<u8>) -> Self {
        Self {
            request_id,
            request_type,
            payload,
            timestamp: Utc::now().timestamp_millis() as u64,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        buf.push(self.request_type as u8);
        buf.extend_from_slice(&self.request_id.to_le_bytes());
        buf.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf.extend_from_slice(&self.timestamp.to_le_bytes());

        buf
    }

    pub fn decode(data: &[u8]) -> Result<Self, String> {
        if data.len() < 17 {
            return Err("Insufficient data for ControlRequest".to_string());
        }

        let request_type =
            RequestType::from_u8(data[0]).ok_or_else(|| "Invalid request type".to_string())?;

        let request_id = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
        let payload_len = u32::from_le_bytes([data[5], data[6], data[7], data[8]]) as usize;

        if data.len() < 17 + payload_len {
            return Err("Payload size mismatch".to_string());
        }

        let payload = data[9..9 + payload_len].to_vec();
        let timestamp =
            u64::from_le_bytes(data[9 + payload_len..17 + payload_len].try_into().unwrap());

        Ok(ControlRequest {
            request_id,
            request_type,
            payload,
            timestamp,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlResponse {
    pub request_id: u32,
    pub status: ResponseStatus,
    pub payload: Vec<u8>,
    pub error_message: String,
    pub timestamp: u64,
}

impl ControlResponse {
    pub fn new(request_id: u32, status: ResponseStatus, payload: Vec<u8>) -> Self {
        Self {
            request_id,
            status,
            payload,
            error_message: String::new(),
            timestamp: Utc::now().timestamp_millis() as u64,
        }
    }

    pub fn error(request_id: u32, message: String) -> Self {
        Self {
            request_id,
            status: ResponseStatus::Error,
            payload: Vec::new(),
            error_message: message,
            timestamp: Utc::now().timestamp_millis() as u64,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        buf.push(self.status as u8);
        buf.extend_from_slice(&self.request_id.to_le_bytes());
        buf.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.payload);

        let error_bytes = self.error_message.as_bytes();
        buf.extend_from_slice(&(error_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(error_bytes);

        buf.extend_from_slice(&self.timestamp.to_le_bytes());

        buf
    }

    pub fn decode(data: &[u8]) -> Result<Self, String> {
        if data.len() < 21 {
            return Err("Insufficient data for ControlResponse".to_string());
        }

        let status = ResponseStatus::from_u8(data[0])
            .ok_or_else(|| "Invalid response status".to_string())?;

        let request_id = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
        let payload_len = u32::from_le_bytes([data[5], data[6], data[7], data[8]]) as usize;

        let payload_end = 9 + payload_len;
        if data.len() < payload_end + 4 {
            return Err("Payload size mismatch".to_string());
        }

        let payload = data[9..payload_end].to_vec();

        let error_len = u32::from_le_bytes([
            data[payload_end],
            data[payload_end + 1],
            data[payload_end + 2],
            data[payload_end + 3],
        ]) as usize;

        let error_end = payload_end + 4 + error_len;
        if data.len() < error_end + 8 {
            return Err("Error message size mismatch".to_string());
        }

        let error_message = String::from_utf8(data[payload_end + 4..error_end].to_vec())
            .map_err(|e| e.to_string())?;

        let timestamp = u64::from_le_bytes(
            data[error_end..error_end + 8]
                .try_into()
                .map_err(|e: std::array::TryFromSliceError| e.to_string())?,
        );

        Ok(ControlResponse {
            request_id,
            status,
            payload,
            error_message,
            timestamp,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub client_id: String,
    pub timestamp: u64,
    pub session_id: String,
}

impl Heartbeat {
    pub fn new(client_id: String, session_id: String) -> Self {
        Self {
            client_id,
            timestamp: Utc::now().timestamp_millis() as u64,
            session_id,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        let client_bytes = self.client_id.as_bytes();
        buf.extend_from_slice(&(client_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(client_bytes);

        buf.extend_from_slice(&self.timestamp.to_le_bytes());

        let session_bytes = self.session_id.as_bytes();
        buf.extend_from_slice(&(session_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(session_bytes);

        buf
    }

    pub fn decode(data: &[u8]) -> Result<Self, String> {
        if data.len() < 16 {
            return Err("Insufficient data for Heartbeat".to_string());
        }

        let client_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let client_end = 4 + client_len;

        if data.len() < client_end + 8 {
            return Err("Client ID size mismatch".to_string());
        }

        let client_id =
            String::from_utf8(data[4..client_end].to_vec()).map_err(|e| e.to_string())?;

        let timestamp = u64::from_le_bytes([
            data[client_end],
            data[client_end + 1],
            data[client_end + 2],
            data[client_end + 3],
            data[client_end + 4],
            data[client_end + 5],
            data[client_end + 6],
            data[client_end + 7],
        ]);

        let session_start = client_end + 8;
        if data.len() < session_start + 4 {
            return Err("Session ID length field missing".to_string());
        }

        let session_len = u32::from_le_bytes([
            data[session_start],
            data[session_start + 1],
            data[session_start + 2],
            data[session_start + 3],
        ]) as usize;

        let session_end = session_start + 4 + session_len;
        if data.len() < session_end {
            return Err("Session ID size mismatch".to_string());
        }

        let session_id = String::from_utf8(data[session_start + 4..session_end].to_vec())
            .map_err(|e| e.to_string())?;

        Ok(Heartbeat {
            client_id,
            timestamp,
            session_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_request_roundtrip() {
        let req = ControlRequest::new(42, RequestType::GetInstruments, vec![1, 2, 3, 4]);
        let encoded = req.encode();
        let decoded = ControlRequest::decode(&encoded).unwrap();

        assert_eq!(decoded.request_id, req.request_id);
        assert_eq!(decoded.request_type, req.request_type);
        assert_eq!(decoded.payload, req.payload);
    }

    #[test]
    fn test_control_response_roundtrip() {
        let resp = ControlResponse::new(42, ResponseStatus::Success, vec![5, 6, 7, 8]);
        let encoded = resp.encode();
        let decoded = ControlResponse::decode(&encoded).unwrap();

        assert_eq!(decoded.request_id, resp.request_id);
        assert_eq!(decoded.status, resp.status);
        assert_eq!(decoded.payload, resp.payload);
    }

    #[test]
    fn test_heartbeat_roundtrip() {
        let hb = Heartbeat::new("client1".to_string(), "session-123".to_string());
        let encoded = hb.encode();
        let decoded = Heartbeat::decode(&encoded).unwrap();

        assert_eq!(decoded.client_id, hb.client_id);
        assert_eq!(decoded.session_id, hb.session_id);
    }
}
