using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;

public sealed class WebpackJob : IDisposable {
    [StructLayout(LayoutKind.Sequential)] struct BasicLimit {
        public long UserTime, JobTime; public uint Flags;
        public UIntPtr MinWorking, MaxWorking; public uint Active;
        public UIntPtr Affinity; public uint Priority, Scheduling;
    }
    [StructLayout(LayoutKind.Sequential)] struct IoCounters {
        public ulong ReadOps, WriteOps, OtherOps, ReadBytes, WriteBytes, OtherBytes;
    }
    [StructLayout(LayoutKind.Sequential)] struct ExtendedLimit {
        public BasicLimit Basic; public IoCounters Io;
        public UIntPtr ProcessMemory, JobMemory, PeakProcess, PeakJob;
    }
    [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)] struct Startup {
        public uint Size; public string Reserved, Desktop, Title;
        public uint X, Y, Width, Height, CharsX, CharsY, Fill, Flags;
        public ushort Show, ReservedSize; public IntPtr ReservedBytes, Input, Output, Error;
    }
    [StructLayout(LayoutKind.Sequential)] struct ProcessInfo {
        public IntPtr Process, Thread; public uint Pid, Tid;
    }
    [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr CreateJobObject(IntPtr attributes, string name);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool SetInformationJobObject(IntPtr job, int kind, ref ExtendedLimit info, uint size);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool QueryInformationJobObject(IntPtr job, int kind, IntPtr info, uint size, IntPtr returned);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool CreateProcess(string app, StringBuilder command, IntPtr pa, IntPtr ta, bool inherit, uint flags, IntPtr env, string cwd, ref Startup startup, out ProcessInfo info);
    [DllImport("kernel32.dll", SetLastError=true)] static extern uint ResumeThread(IntPtr thread);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool TerminateProcess(IntPtr process, uint code);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool TerminateJobObject(IntPtr job, uint code);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool GetExitCodeProcess(IntPtr process, out uint code);
    [DllImport("kernel32.dll", SetLastError=true)] static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll")] static extern IntPtr GetStdHandle(int kind);
    readonly HashSet<uint> observed = new HashSet<uint>();
    IntPtr job, root;
    public uint RootPid { get; private set; }
    public uint[] Observed { get { var result = new uint[observed.Count]; observed.CopyTo(result); return result; } }
    static void Check(bool success) { if (!success) throw new Win32Exception(Marshal.GetLastWin32Error()); }
    public WebpackJob() {
        job = CreateJobObject(IntPtr.Zero, null);
        if (job == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
        var limit = new ExtendedLimit(); limit.Basic.Flags = 0x2000;
        try { Check(SetInformationJobObject(job, 9, ref limit, (uint)Marshal.SizeOf(limit))); }
        catch { CloseHandle(job); job = IntPtr.Zero; throw; }
    }
    public void Launch(string command, string cwd) {
        var startup = new Startup(); startup.Size = (uint)Marshal.SizeOf(startup);
        startup.Flags = 0x100; startup.Input = GetStdHandle(-10);
        startup.Output = GetStdHandle(-11); startup.Error = GetStdHandle(-12);
        ProcessInfo info;
        Check(CreateProcess(null, new StringBuilder(command), IntPtr.Zero, IntPtr.Zero, true, 4, IntPtr.Zero, cwd, ref startup, out info));
        try {
            Check(AssignProcessToJobObject(job, info.Process));
            root = info.Process; RootPid = info.Pid; observed.Add(info.Pid);
            if (ResumeThread(info.Thread) == UInt32.MaxValue) throw new Win32Exception(Marshal.GetLastWin32Error());
        } catch {
            TerminateProcess(info.Process, 125);
            WaitForSingleObject(info.Process, 10000);
            if (root == IntPtr.Zero) CloseHandle(info.Process);
            throw;
        } finally { CloseHandle(info.Thread); }
    }
    public bool RootExited { get { return root != IntPtr.Zero && WaitForSingleObject(root, 0) == 0; } }
    public uint ExitCode { get { uint code; Check(GetExitCodeProcess(root, out code)); return code; } }
    public uint[] Members() {
        int capacity = 64; IntPtr buffer;
        while (true) {
            buffer = Marshal.AllocHGlobal(8 + capacity * IntPtr.Size);
            if (QueryInformationJobObject(job, 3, buffer, (uint)(8 + capacity * IntPtr.Size), IntPtr.Zero)) break;
            int error = Marshal.GetLastWin32Error(); Marshal.FreeHGlobal(buffer);
            if (error != 234 || capacity >= 1048576) throw new Win32Exception(error);
            capacity *= 2;
        }
        try {
            int count = Marshal.ReadInt32(buffer, 4);
            var result = new uint[count];
            for (int i = 0; i < count; i++) {
                result[i] = checked((uint)Marshal.ReadIntPtr(buffer, 8 + i * IntPtr.Size).ToInt64());
                observed.Add(result[i]);
            }
            return result;
        } finally { Marshal.FreeHGlobal(buffer); }
    }
    public void KillAndWait() {
        Members(); Check(TerminateJobObject(job, 125));
        var clock = System.Diagnostics.Stopwatch.StartNew();
        while (Members().Length != 0) {
            if (clock.ElapsedMilliseconds >= 10000) throw new TimeoutException("Job cleanup exceeded 10s");
            System.Threading.Thread.Sleep(20);
        }
        if (root != IntPtr.Zero && WaitForSingleObject(root, 1000) != 0) throw new TimeoutException("Root not reaped");
    }
    public void Dispose() {
        if (job != IntPtr.Zero) { Check(CloseHandle(job)); job = IntPtr.Zero; }
        if (root != IntPtr.Zero) { Check(CloseHandle(root)); root = IntPtr.Zero; }
    }
}
