# Hardware-in-the-Loop (HIL) Testing Setup

This document outlines the setup for running hardware-in-the-loop (HIL) tests using GitHub Actions and Tailscale.

## Overview

The `.github/workflows/hardware-in-the-loop.yml` workflow enables GitHub Actions runners to securely connect to physical hardware on our private Tailscale network (tailnet). This allows us to run integration tests that interact directly with our instruments.

The workflow performs the following steps:
1.  Checks out the repository code.
2.  Connects the runner to the tailnet using the `tailscale/github-action@v2` action.
3.  Verifies connectivity by checking the Tailscale status and pinging a designated hardware device.
4.  Runs a placeholder step for the actual HIL tests.

## Setup and Configuration

To use this workflow, you need to configure a Tailscale auth key as a GitHub secret.

### 1. Generate a Tailscale Auth Key

The workflow uses a tagged, ephemeral auth key to connect to the tailnet. This ensures that the access is temporary and restricted.

To generate a key:
1.  Go to the **Keys** page in the Tailscale admin console.
2.  Click **Generate auth key...**.
3.  Configure the key with the following settings:
    -   **Reusable**: No (or Yes, if you need to reuse it for a short period, but ephemeral is safer).
    -   **Ephemeral**: Yes. This ensures the device is removed from the tailnet after it disconnects.
    -   **Tags**: `tag:ci`. This tag is used to apply specific Access Control Lists (ACLs) to the CI runners.
    -   **Pre-authorized**: Yes. This allows the runner to join the tailnet without manual approval.
4.  Click **Generate key**.
5.  Copy the generated key (`tskey-...`). **You will not be able to see this key again.**

### 2. Add the Auth Key to GitHub Secrets

The auth key must be stored as an encrypted secret in the GitHub repository.

1.  In the GitHub repository, go to **Settings** > **Secrets and variables** > **Actions**.
2.  Click **New repository secret**.
3.  Set the **Name** to `TS_AUTHKEY`.
4.  Paste the generated Tailscale auth key into the **Value** field.
5.  Click **Add secret**.

### 3. Configure Tailscale ACLs

Ensure that your tailnet's ACLs grant the `tag:ci` the necessary permissions to access the hardware devices. For example:

```json
{
  "acls": [
    // Allow CI runners to access the hardware device
    {
      "action": "accept",
      "src": ["tag:ci"],
      "dst": ["your-hardware-device-name:22"] // Example: allow SSH access
    }
  ]
}
```

Replace `your-hardware-device-name` with the actual name or tag of your hardware device in Tailscale.

## Running HIL Tests

To add HIL tests, modify the `hardware-in-the-loop.yml` workflow file:

1.  Locate the placeholder step:
    ```yaml
          - name: Run hardware-in-the-loop tests
            run: |
              echo "Running hardware-in-the-loop tests..."
              # Add your test commands here.
    ```
2.  Replace the `echo` command with the actual commands needed to run your tests. These commands can now access the hardware on the tailnet.
3.  Remember to replace `'your-hardware-device-name'` in the ping step with the actual device name.