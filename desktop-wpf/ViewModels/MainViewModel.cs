// RemoteDesk Free - Main Application ViewModel
// Copyright (C) 2026 Suresh Mamidala | Freeware

using System;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Windows.Input;
using RemoteDesk.Wpf.Interop;

namespace RemoteDesk.Wpf.ViewModels
{
    public class RecentDeviceItem
    {
        public string DeviceId { get; set; } = "";
        public string Alias { get; set; } = "";
        public string LastSeen { get; set; } = "";
        public bool IsFavorite { get; set; }
    }

    public class SessionTabItem : INotifyPropertyChanged
    {
        private string _title = "";
        private bool _isActive;

        public string PeerId { get; set; } = "";
        public string Title
        {
            get => _title;
            set { _title = value; OnPropertyChanged(); }
        }
        public bool IsActive
        {
            get => _isActive;
            set { _isActive = value; OnPropertyChanged(); }
        }

        public event PropertyChangedEventHandler? PropertyChanged;
        protected void OnPropertyChanged([CallerMemberName] string? name = null) =>
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
    }

    public class MainViewModel : INotifyPropertyChanged
    {
        private string _myDeviceId = "849 203 918";
        private string _myPin = "582 910";
        private string _targetDeviceId = "";
        private string _targetPin = "";
        private string _unattendedPassword = "";
        private string _statusMessage = "Ready • Global Rendezvous Active (No Geo Limits)";
        private bool _isConnecting;
        private bool _isInSession;
        private string _telemetryFps = "60 FPS";
        private string _telemetryLatency = "< 15 ms";
        private string _telemetryQuality = "GPU H.264 • 1080p";
        private string _telemetryTransport = "WebRTC P2P (Direct UDP)";

        public string MyDeviceId
        {
            get => _myDeviceId;
            set { _myDeviceId = value; OnPropertyChanged(); }
        }

        public string MyPin
        {
            get => _myPin;
            set { _myPin = value; OnPropertyChanged(); }
        }

        public string TargetDeviceId
        {
            get => _targetDeviceId;
            set { _targetDeviceId = value; OnPropertyChanged(); }
        }

        public string TargetPin
        {
            get => _targetPin;
            set { _targetPin = value; OnPropertyChanged(); }
        }

        public string UnattendedPassword
        {
            get => _unattendedPassword;
            set { _unattendedPassword = value; OnPropertyChanged(); }
        }

        public string StatusMessage
        {
            get => _statusMessage;
            set { _statusMessage = value; OnPropertyChanged(); }
        }

        public bool IsConnecting
        {
            get => _isConnecting;
            set { _isConnecting = value; OnPropertyChanged(); }
        }

        public bool IsInSession
        {
            get => _isInSession;
            set { _isInSession = value; OnPropertyChanged(); }
        }

        public string TelemetryFps
        {
            get => _telemetryFps;
            set { _telemetryFps = value; OnPropertyChanged(); }
        }

        public string TelemetryLatency
        {
            get => _telemetryLatency;
            set { _telemetryLatency = value; OnPropertyChanged(); }
        }

        public string TelemetryQuality
        {
            get => _telemetryQuality;
            set { _telemetryQuality = value; OnPropertyChanged(); }
        }

        public string TelemetryTransport
        {
            get => _telemetryTransport;
            set { _telemetryTransport = value; OnPropertyChanged(); }
        }

        public ObservableCollection<SessionTabItem> ActiveSessions { get; } = new();
        public ObservableCollection<RecentDeviceItem> RecentDevices { get; } = new();

        public NativeCaptureEngine? HostCaptureEngine { get; private set; }
        public NativeVideoDecoder? ClientDecoder { get; private set; }

        public MainViewModel()
        {
            // Seed sample recent devices for modern Address Book
            RecentDevices.Add(new RecentDeviceItem { DeviceId = "482 910 374", Alias = "Office Workstation", LastSeen = "Today 10:45 AM", IsFavorite = true });
            RecentDevices.Add(new RecentDeviceItem { DeviceId = "739 184 205", Alias = "Production Server 01", LastSeen = "Yesterday 04:20 PM", IsFavorite = true });
            RecentDevices.Add(new RecentDeviceItem { DeviceId = "302 918 472", Alias = "Remote Laptop (Support)", LastSeen = "3 days ago", IsFavorite = false });
        }

        public void ConnectToRemote()
        {
            if (string.IsNullOrWhiteSpace(TargetDeviceId))
            {
                StatusMessage = "Please enter a valid 9-digit Remote Desk ID.";
                return;
            }

            IsConnecting = true;
            StatusMessage = $"Connecting to {TargetDeviceId} via Native WebRTC P2P Direct UDP...";

            // Simulate instant high-speed rendezvous and session establishment
            string cleanId = TargetDeviceId.Trim();
            ActiveSessions.Clear();
            ActiveSessions.Add(new SessionTabItem { PeerId = cleanId, Title = $"Connected: {cleanId}", IsActive = true });

            IsInSession = true;
            IsConnecting = false;
            StatusMessage = "Session Established • Hardware GPU H.264 Pipeline Active";
            TelemetryFps = "60 FPS";
            TelemetryLatency = "12 ms";
            TelemetryQuality = "GPU H.264 • 1920x1080";
            TelemetryTransport = "WebRTC P2P (Direct UDP)";
        }

        public void DisconnectSession()
        {
            IsInSession = false;
            ActiveSessions.Clear();
            StatusMessage = "Remote session ended.";
        }

        public void RequestKeyframe()
        {
            HostCaptureEngine?.RequestKeyframe();
            StatusMessage = "Requested instant IDR Keyframe from host.";
        }

        public event PropertyChangedEventHandler? PropertyChanged;
        protected void OnPropertyChanged([CallerMemberName] string? name = null) =>
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
    }
}
