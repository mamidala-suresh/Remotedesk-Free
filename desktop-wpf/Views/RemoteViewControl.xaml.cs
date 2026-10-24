// RemoteDesk Free - Remote Screen Viewport & Input Handler
// Copyright (C) 2026 Suresh Mamidala | Freeware

using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using RemoteDesk.Wpf.Interop;
using RemoteDesk.Wpf.Rendering;
using RemoteDesk.Wpf.ViewModels;

namespace RemoteDesk.Wpf.Views
{
    public partial class RemoteViewControl : UserControl
    {
        private D3D11ImageSource? _d3dImage;

        public RemoteViewControl()
        {
            InitializeComponent();
            Loaded += OnLoaded;
            Unloaded += OnUnloaded;
        }

        private void OnLoaded(object sender, RoutedEventArgs e)
        {
            _d3dImage = new D3D11ImageSource();
            ScreenImage.Source = _d3dImage;
            Focus();
        }

        private void OnUnloaded(object sender, RoutedEventArgs e)
        {
            _d3dImage?.Dispose();
            _d3dImage = null;
        }

        private (int x, int y, uint w, uint h) GetNormalizedCoordinates(MouseEventArgs e)
        {
            Point pos = e.GetPosition(ScreenImage);
            double actualW = ScreenImage.ActualWidth;
            double actualH = ScreenImage.ActualHeight;

            if (actualW <= 0 || actualH <= 0) return (0, 0, 1920, 1080);

            uint nativeW = _d3dImage?.FrameWidth > 0 ? _d3dImage.FrameWidth : 1920;
            uint nativeH = _d3dImage?.FrameHeight > 0 ? _d3dImage.FrameHeight : 1080;

            int mappedX = (int)Math.Clamp((pos.X / actualW) * nativeW, 0, nativeW);
            int mappedY = (int)Math.Clamp((pos.Y / actualH) * nativeH, 0, nativeH);

            return (mappedX, mappedY, nativeW, nativeH);
        }

        private void OnScreenMouseMove(object sender, MouseEventArgs e)
        {
            var (x, y, w, h) = GetNormalizedCoordinates(e);
            NativeMethods.rd_input_mouse_move(x, y, w, h);
        }

        private void OnScreenMouseDown(object sender, MouseButtonEventArgs e)
        {
            Focus();
            var (x, y, w, h) = GetNormalizedCoordinates(e);
            RdMouseButton btn = e.ChangedButton switch
            {
                MouseButton.Right => RdMouseButton.Right,
                MouseButton.Middle => RdMouseButton.Middle,
                MouseButton.XButton1 => RdMouseButton.X1,
                MouseButton.XButton2 => RdMouseButton.X2,
                _ => RdMouseButton.Left
            };
            NativeMethods.rd_input_mouse_down(btn, x, y, w, h);
        }

        private void OnScreenMouseUp(object sender, MouseButtonEventArgs e)
        {
            var (x, y, w, h) = GetNormalizedCoordinates(e);
            RdMouseButton btn = e.ChangedButton switch
            {
                MouseButton.Right => RdMouseButton.Right,
                MouseButton.Middle => RdMouseButton.Middle,
                MouseButton.XButton1 => RdMouseButton.X1,
                MouseButton.XButton2 => RdMouseButton.X2,
                _ => RdMouseButton.Left
            };
            NativeMethods.rd_input_mouse_up(btn, x, y, w, h);
        }

        private void OnScreenMouseWheel(object sender, MouseWheelEventArgs e)
        {
            NativeMethods.rd_input_mouse_wheel(0, e.Delta);
        }

        protected override void OnPreviewKeyDown(KeyEventArgs e)
        {
            base.OnPreviewKeyDown(e);
            Key actualKey = e.Key == Key.System ? e.SystemKey : e.Key;
            int vk = KeyInterop.VirtualKeyFromKey(actualKey);
            bool isExtended = (vk >= 0x21 && vk <= 0x28) || vk == 0x2D || vk == 0x2E || vk == 0x5B || vk == 0x5C || vk == 0x5D || vk == 0xA3 || vk == 0xA5;
            NativeMethods.rd_input_key_down((uint)vk, 0, isExtended);
            e.Handled = true;
        }

        protected override void OnPreviewKeyUp(KeyEventArgs e)
        {
            base.OnPreviewKeyUp(e);
            Key actualKey = e.Key == Key.System ? e.SystemKey : e.Key;
            int vk = KeyInterop.VirtualKeyFromKey(actualKey);
            bool isExtended = (vk >= 0x21 && vk <= 0x28) || vk == 0x2D || vk == 0x2E || vk == 0x5B || vk == 0x5C || vk == 0x5D || vk == 0xA3 || vk == 0xA5;
            NativeMethods.rd_input_key_up((uint)vk, 0, isExtended);
            e.Handled = true;
        }

        private void OnSendRunDialogClick(object sender, RoutedEventArgs e)
        {
            NativeMethods.rd_input_key_down(0x5B, 0, true); // VK_LWIN
            NativeMethods.rd_input_key_down(0x52, 0, false); // VK_R
            System.Threading.Thread.Sleep(20);
            NativeMethods.rd_input_key_up(0x52, 0, false);
            NativeMethods.rd_input_key_up(0x5B, 0, true);
        }

        private void OnRequestKeyframeClick(object sender, RoutedEventArgs e)
        {
            if (DataContext is MainViewModel vm)
            {
                vm.RequestKeyframe();
            }
        }

        private void OnSendCtrlAltDelClick(object sender, RoutedEventArgs e)
        {
            NativeMethods.rd_input_key_down(0x11, 0, false); // VK_CONTROL
            NativeMethods.rd_input_key_down(0x12, 0, false); // VK_MENU (Alt)
            NativeMethods.rd_input_key_down(0x2E, 0, true);  // VK_DELETE
            NativeMethods.rd_input_key_up(0x2E, 0, true);
            NativeMethods.rd_input_key_up(0x12, 0, false);
            NativeMethods.rd_input_key_up(0x11, 0, false);
        }

        private void OnSendTaskManagerClick(object sender, RoutedEventArgs e)
        {
            NativeMethods.rd_input_key_down(0x11, 0, false); // VK_CONTROL
            NativeMethods.rd_input_key_down(0x10, 0, false); // VK_SHIFT
            NativeMethods.rd_input_key_down(0x1B, 0, false); // VK_ESCAPE
            NativeMethods.rd_input_key_up(0x1B, 0, false);
            NativeMethods.rd_input_key_up(0x10, 0, false);
            NativeMethods.rd_input_key_up(0x11, 0, false);
        }

        private void OnDisconnectClick(object sender, RoutedEventArgs e)
        {
            if (DataContext is MainViewModel vm)
            {
                vm.DisconnectSession();
            }
        }
    }
}
