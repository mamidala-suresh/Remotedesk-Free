// RemoteDesk Free - WPF Application Entrypoint
// Copyright (C) 2026 Suresh Mamidala | Freeware

using System;
using System.Windows;

namespace RemoteDesk.Wpf
{
    public partial class App : Application
    {
        protected override void OnStartup(StartupEventArgs e)
        {
            base.OnStartup(e);

            AppDomain.CurrentDomain.UnhandledException += (s, args) =>
            {
                MessageBox.Show($"Unexpected error: {args.ExceptionObject}", "RemoteDesk Free Error", MessageBoxButton.OK, MessageBoxImage.Error);
            };
        }
    }
}
