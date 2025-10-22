#[cfg(feature = "networking")]
mod network_protocol_tests {
    use rust_daq::network::protocol::{ControlRequest, ControlResponse, Heartbeat, RequestType, ResponseStatus};

    #[test]
    fn test_control_request_encode_decode() {
        let original = ControlRequest::new(
            42,
            RequestType::GetInstruments,
            vec![1, 2, 3, 4, 5],
        );

        let encoded = original.encode();
        let decoded = ControlRequest::decode(&encoded).expect("Failed to decode");

        assert_eq!(decoded.request_id, original.request_id);
        assert_eq!(decoded.request_type, original.request_type);
        assert_eq!(decoded.payload, original.payload);
    }

    #[test]
    fn test_control_response_encode_decode() {
        let original = ControlResponse::new(
            123,
            ResponseStatus::Success,
            vec![10, 20, 30],
        );

        let encoded = original.encode();
        let decoded = ControlResponse::decode(&encoded).expect("Failed to decode");

        assert_eq!(decoded.request_id, original.request_id);
        assert_eq!(decoded.status, original.status);
        assert_eq!(decoded.payload, original.payload);
    }

    #[test]
    fn test_control_response_error() {
        let error_resp = ControlResponse::error(
            456,
            "Something went wrong".to_string(),
        );

        let encoded = error_resp.encode();
        let decoded = ControlResponse::decode(&encoded).expect("Failed to decode");

        assert_eq!(decoded.request_id, 456);
        assert_eq!(decoded.status, ResponseStatus::Error);
        assert_eq!(decoded.error_message, "Something went wrong");
    }

    #[test]
    fn test_heartbeat_encode_decode() {
        let original = Heartbeat::new(
            "client-192.168.1.100:12345".to_string(),
            "sess-uuid-1234".to_string(),
        );

        let encoded = original.encode();
        let decoded = Heartbeat::decode(&encoded).expect("Failed to decode");

        assert_eq!(decoded.client_id, original.client_id);
        assert_eq!(decoded.session_id, original.session_id);
    }

    #[test]
    fn test_various_request_types() {
        let request_types = vec![
            RequestType::GetInstruments,
            RequestType::StartRecording,
            RequestType::StopRecording,
            RequestType::SpawnInstrument,
            RequestType::StopInstrument,
            RequestType::SendCommand,
            RequestType::Heartbeat,
        ];

        for req_type in request_types {
            let req = ControlRequest::new(
                100,
                req_type,
                vec![1, 2, 3],
            );

            let encoded = req.encode();
            let decoded = ControlRequest::decode(&encoded).expect("Failed to decode");

            assert_eq!(decoded.request_type, req_type, "Failed for {:?}", req_type);
        }
    }

    #[test]
    fn test_empty_payload() {
        let req = ControlRequest::new(
            1,
            RequestType::Heartbeat,
            vec![],
        );

        let encoded = req.encode();
        let decoded = ControlRequest::decode(&encoded).expect("Failed to decode");

        assert_eq!(decoded.payload.len(), 0);
    }

    #[test]
    fn test_large_payload() {
        let large_payload = vec![0xAB; 1024 * 10];
        let req = ControlRequest::new(
            1,
            RequestType::SendCommand,
            large_payload.clone(),
        );

        let encoded = req.encode();
        let decoded = ControlRequest::decode(&encoded).expect("Failed to decode");

        assert_eq!(decoded.payload, large_payload);
    }

    #[test]
    fn test_malformed_request() {
        let malformed = vec![0xFF, 0xFF];
        let result = ControlRequest::decode(&malformed);
        assert!(result.is_err());
    }

    #[test]
    fn test_response_with_error_message() {
        let error_message = "Failed to connect to instrument: timeout after 5 seconds".to_string();
        let resp = ControlResponse::error(999, error_message.clone());

        let encoded = resp.encode();
        let decoded = ControlResponse::decode(&encoded).expect("Failed to decode");

        assert_eq!(decoded.error_message, error_message);
        assert_eq!(decoded.status, ResponseStatus::Error);
    }
}
