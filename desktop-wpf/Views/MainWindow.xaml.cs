// RemoteDesk Free - Main Window Code Behind
// Copyright (C) 2026 Suresh Mamidala | Freeware

using System.Windows;
using System.Windows.Controls;
using RemoteDesk.Wpf.ViewModels;

namespace RemoteDesk.Wpf.Views
{
    public partial class MainWindow : Window
    {
        public MainWindow()
        {
            InitializeComponent();
        }

        private void OnConnectClick(object sender, RoutedEventArgs e)
        {
            if (DataContext is MainViewModel vm)
            {
                vm.ConnectToRemote();
            }
        }

        private void OnRecentConnectClick(object sender, RoutedEventArgs e)
        {
            if (sender is Button btn && btn.DataContext is RecentDeviceItem item && DataContext is MainViewModel vm)
            {
                vm.TargetDeviceId = item.DeviceId;
                vm.ConnectToRemote();
            }
        }
    }
}
