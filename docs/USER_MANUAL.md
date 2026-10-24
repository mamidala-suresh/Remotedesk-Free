# RemoteDesk Free - User Manual & Operation Guide

## 1. Getting Started

When you launch `remotedesk-app.exe`, the Home Dashboard presents two primary cards:
- **This Desk (Left Pane)**: Shows your 9-digit Device ID (e.g., `482 910 231`) and a 6-digit one-time session PIN.
- **Remote Desk (Right Pane)**: Allows you to input a remote computer's 9-digit ID to initiate a connection.

---

## 2. Allowing Someone to Connect to Your Computer (Interactive Mode)

1. Provide your 9-digit Device ID to the technician or colleague.
2. When the remote user initiates a connection, a popup modal will appear on your screen:
   - Displays the remote user's Computer Name, Device ID, and IP address.
   - You can review and adjust granted permissions (View screen, Control mouse/keyboard, Clipboard synchronization, File transfer).
3. Click **Accept Connection** to grant access, or **Decline Request** to refuse access immediately.
4. You can disconnect the session at any moment.

---

## 3. Connecting to a Remote Computer

1. Enter the remote computer's 9-digit ID into the **Target Device ID** field.
2. (Optional) Under **Authentication & Options**, enter the one-time PIN or master unattended password if required.
3. Click **Connect**.
4. Once accepted, the remote desktop will stream on your screen in real time.

---

## 4. In-Session Toolbar Features

- **Monitor Selector**: If the remote computer has multiple displays, switch between Display 1, Display 2, etc.
- **Scaling Mode**: Choose between **Scale to Fit**, **Original (1:1)**, or **Stretch**.
- **Full Screen**: Toggle distraction-free full-screen display.
- **Chat**: Open the real-time chat drawer to text the remote user.
- **File Manager**: Launch the dual-pane file transfer utility to upload or download files.
- **Disconnect**: Immediately terminates the remote session.

---

## 5. Configuring Unattended Access (For IT Administrators)

1. In the top navigation bar, click **Security & Audit**.
2. Check the box **Enable Unattended Access on This Machine**.
3. Enter and confirm a strong master password (minimum 8 characters).
4. Click **Save Unattended Password**.
5. Authorized technicians can now connect remotely using this password without requiring manual confirmation on the physical monitor.
