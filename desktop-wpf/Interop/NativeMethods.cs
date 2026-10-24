// RemoteDesk Free - Native C++ Engine Interop Bindings
// Copyright (C) 2026 Suresh Mamidala | Freeware

using System;
using System.Runtime.InteropServices;

namespace RemoteDesk.Wpf.Interop
{
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Ansi)]
    public struct RdMonitorInfo
    {
        public uint Index;
        public uint Width;
        public uint Height;
        public int Left;
        public int Top;
        [MarshalAs(UnmanagedType.I1)]
        public bool IsPrimary;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 64)]
        public string DeviceName;
    }

    public enum RdMouseButton : int
    {
        Left = 1,
        Right = 2,
        Middle = 3,
        X1 = 4,
        X2 = 5
    }

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate void RdEncodedFrameCallback(
        IntPtr data,
        uint size,
        uint width,
        uint height,
        [MarshalAs(UnmanagedType.I1)] bool isKeyframe,
        ulong timestampMs,
        IntPtr userData
    );

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate void RdDecodedFrameCallback(
        IntPtr d3d11TextureHandle,
        uint width,
        uint height,
        IntPtr userData
    );

    public static class NativeMethods
    {
        private const string DllName = "remotedesk_native.dll";

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int rd_get_monitors([Out] RdMonitorInfo[] monitors, int maxCount);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr rd_engine_create(
            uint monitorIndex,
            uint targetFps,
            uint targetBitrateKbps,
            uint scalePercent,
            RdEncodedFrameCallback callback,
            IntPtr userData
        );

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        [return: MarshalAs(UnmanagedType.I1)]
        public static extern bool rd_engine_start(IntPtr engine);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        [return: MarshalAs(UnmanagedType.I1)]
        public static extern bool rd_engine_stop(IntPtr engine);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void rd_engine_request_keyframe(IntPtr engine);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void rd_engine_set_quality(IntPtr engine, uint bitrateKbps, uint targetFps, uint scalePercent);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void rd_engine_destroy(IntPtr engine);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr rd_decoder_create(RdDecodedFrameCallback callback, IntPtr userData);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        [return: MarshalAs(UnmanagedType.I1)]
        public static extern bool rd_decoder_decode(IntPtr decoder, IntPtr h264Data, uint size, ulong timestampMs);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr rd_decoder_get_shared_handle(IntPtr decoder);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void rd_decoder_destroy(IntPtr decoder);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        [return: MarshalAs(UnmanagedType.I1)]
        public static extern bool rd_input_mouse_move(int x, int y, uint screenW, uint screenH);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        [return: MarshalAs(UnmanagedType.I1)]
        public static extern bool rd_input_mouse_down(RdMouseButton button, int x, int y, uint screenW, uint screenH);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        [return: MarshalAs(UnmanagedType.I1)]
        public static extern bool rd_input_mouse_up(RdMouseButton button, int x, int y, uint screenW, uint screenH);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        [return: MarshalAs(UnmanagedType.I1)]
        public static extern bool rd_input_mouse_wheel(int deltaX, int deltaY);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        [return: MarshalAs(UnmanagedType.I1)]
        public static extern bool rd_input_key_down(uint vkCode, uint scanCode, [MarshalAs(UnmanagedType.I1)] bool extended);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl)]
        [return: MarshalAs(UnmanagedType.I1)]
        public static extern bool rd_input_key_up(uint vkCode, uint scanCode, [MarshalAs(UnmanagedType.I1)] bool extended);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
        [return: MarshalAs(UnmanagedType.I1)]
        public static extern bool rd_input_text_utf16(string text);
    }

    /// <summary>
    /// Safe managed wrapper around the C++ DXGI & Hardware H.264 Capture Engine
    /// </summary>
    public sealed class NativeCaptureEngine : IDisposable
    {
        private IntPtr _handle;
        private readonly RdEncodedFrameCallback _nativeCallback;

        public event Action<byte[], uint, uint, bool, ulong>? FrameEncoded;

        public NativeCaptureEngine(uint monitorIndex = 0, uint targetFps = 60, uint bitrateKbps = 3500, uint scalePercent = 100)
        {
            _nativeCallback = OnNativeFrameEncoded;
            _handle = NativeMethods.rd_engine_create(monitorIndex, targetFps, bitrateKbps, scalePercent, _nativeCallback, IntPtr.Zero);
            if (_handle == IntPtr.Zero)
            {
                throw new InvalidOperationException("Failed to initialize native DXGI + GPU H.264 engine.");
            }
        }

        public bool Start() => _handle != IntPtr.Zero && NativeMethods.rd_engine_start(_handle);
        public bool Stop() => _handle != IntPtr.Zero && NativeMethods.rd_engine_stop(_handle);
        public void RequestKeyframe()
        {
            if (_handle != IntPtr.Zero) NativeMethods.rd_engine_request_keyframe(_handle);
        }

        public void SetQuality(uint bitrateKbps, uint targetFps, uint scalePercent)
        {
            if (_handle != IntPtr.Zero) NativeMethods.rd_engine_set_quality(_handle, bitrateKbps, targetFps, scalePercent);
        }

        private void OnNativeFrameEncoded(IntPtr data, uint size, uint width, uint height, bool isKeyframe, ulong timestampMs, IntPtr userData)
        {
            if (data == IntPtr.Zero || size == 0) return;
            byte[] managedBuffer = new byte[size];
            Marshal.Copy(data, managedBuffer, 0, (int)size);
            FrameEncoded?.Invoke(managedBuffer, width, height, isKeyframe, timestampMs);
        }

        public void Dispose()
        {
            if (_handle != IntPtr.Zero)
            {
                NativeMethods.rd_engine_destroy(_handle);
                _handle = IntPtr.Zero;
            }
        }
    }

    /// <summary>
    /// Safe managed wrapper around the C++ Hardware H.264 Video Decoder & DirectX Surface Bridge
    /// </summary>
    public sealed class NativeVideoDecoder : IDisposable
    {
        private IntPtr _handle;
        private readonly RdDecodedFrameCallback _nativeCallback;

        public event Action<IntPtr, uint, uint>? SurfaceReady;

        public NativeVideoDecoder()
        {
            _nativeCallback = OnNativeSurfaceReady;
            _handle = NativeMethods.rd_decoder_create(_nativeCallback, IntPtr.Zero);
            if (_handle == IntPtr.Zero)
            {
                throw new InvalidOperationException("Failed to initialize native Hardware H.264 Decoder.");
            }
        }

        public bool Decode(byte[] h264Data, ulong timestampMs)
        {
            if (_handle == IntPtr.Zero || h264Data == null || h264Data.Length == 0) return false;
            unsafe
            {
                fixed (byte* p = h264Data)
                {
                    return NativeMethods.rd_decoder_decode(_handle, (IntPtr)p, (uint)h264Data.Length, timestampMs);
                }
            }
        }

        public IntPtr GetSharedHandle() => _handle != IntPtr.Zero ? NativeMethods.rd_decoder_get_shared_handle(_handle) : IntPtr.Zero;

        private void OnNativeSurfaceReady(IntPtr d3d11TextureHandle, uint width, uint height, IntPtr userData)
        {
            SurfaceReady?.Invoke(d3d11TextureHandle, width, height);
        }

        public void Dispose()
        {
            if (_handle != IntPtr.Zero)
            {
                NativeMethods.rd_decoder_destroy(_handle);
                _handle = IntPtr.Zero;
            }
        }
    }
}
