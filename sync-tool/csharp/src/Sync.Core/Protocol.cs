using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using static Sync.Core.Prelude;

namespace Sync.Core;

public static class Protocol
{
    // This class implements the client-server protocol
    // It looks like this:
    //
    //        |-------- |---------|----------------|----------|
    // Field: | Header  | Package | ?Optional body | Checksum |
    //  Size: | 32b     | static  | variable       | 16b      |
    //        |-------- |---------|----------------|----------|
    //
    // Static above means it is type dependent (probably could also be fixed to 32bytes)

    static Protocol()
    {
        Assert(BitConverter.IsLittleEndian, "Protocol relies on the same endianness between client-server");
    }

    public static int SizeFor<T>()
        where T : unmanaged, IPackage<T>
    {
        return PackageHeader.Size + T.Size + ChecksumPackage.Size;
    }

    public static void Serialize<T>(ReadOnlySpan<byte> buffer, ref PackageHeader header, ref T package)
        where T : unmanaged, IPackage<T>
    {
        Assert(buffer.Length == SizeFor<T>(), "Buffer should be sized for package");
        header.Serialize(buffer);
        package.Serialize(buffer.Slice(PackageHeader.Size));

        var message = buffer.Slice(0, PackageHeader.Size + T.Size);
        var checksumPackage = new ChecksumPackage { Checksum = Checksummer.Compute(message) };
        checksumPackage.Serialize(buffer.Slice(PackageHeader.Size + T.Size));
    }

    public static bool TryParseHeader(ReadOnlySpan<byte> buffer, out PackageHeader header) =>
        PackageHeader.TryParse(buffer, out header);

    public static bool TryParsePackage<T>(
        ReadOnlySpan<byte> buffer,
        ref PackageHeader header,
        out T package,
        out UInt128 checksum
    )
        where T : unmanaged, IPackage<T>
    {
        checksum = 0;
        if (!T.TryParse(buffer.Slice(PackageHeader.Size), ref header, out package))
            return false;

        if (!ChecksumPackage.TryParse(buffer.Slice(PackageHeader.Size + T.Size), ref header, out var checksumPackage))
            return false;

        checksum = checksumPackage.Checksum;
        var computedChecksum = Checksummer.Compute(buffer.Slice(0, PackageHeader.Size + T.Size));
        // Should really just signal retry operations for stuff like this
        // but since this is just a toy project we'll make an assertion for now.
        // If we were to support retries, we would need to alter the return type to communicate what failed. Cases:
        // * not enough data (yet), so issue more reads
        // * checksum verification failed, sender should retry (we would need another package)
        Assert(computedChecksum == checksum, "Data corruption occurred");

        return true;
    }
}

public enum PackageType : byte
{
    None,
    Session,
    Ok,
}

[StructLayout(LayoutKind.Explicit, Size = Size, Pack = 8)]
public struct PackageHeader
{
    public const int Size = 32;

    [FieldOffset(0)]
    public ulong PartitionId;

    [FieldOffset(8)]
    public ulong MessageId;

    [FieldOffset(16)]
    public byte PackageType;

    public void Serialize(ReadOnlySpan<byte> buffer)
    {
        int size;
        unsafe
        {
            size = sizeof(PackageHeader);
        }
        Assert(size == Size, "The header should have an expected size");
        Assert(buffer.Length >= size, "Buffer must have space for message");
        ref var payload = ref Unsafe.As<PackageHeader, byte>(ref this);

        Unsafe.CopyBlock(ref MemoryMarshal.GetReference(buffer), ref payload, (uint)size);
    }

    public static bool TryParse(ReadOnlySpan<byte> buffer, out PackageHeader header)
    {
        header = default;
        int size;
        unsafe
        {
            size = sizeof(PackageHeader);
        }
        Assert(size == Size, "The header should occupy a cache line");
        if (buffer.Length < size)
            return false;

        Unsafe.CopyBlock(
            ref Unsafe.As<PackageHeader, byte>(ref header),
            ref MemoryMarshal.GetReference(buffer),
            (uint)size
        );

        return true;
    }
}

public interface IPackage<T>
    where T : unmanaged
{
    static abstract int Size { get; }

    void Serialize(ReadOnlySpan<byte> buffer);

    static abstract bool TryParse(ReadOnlySpan<byte> buffer, ref PackageHeader header, out T package);
}

[StructLayout(LayoutKind.Explicit, Size = Size, Pack = 16)]
public struct ChecksumPackage : IPackage<ChecksumPackage>
{
    public const int Size = 16;

    static int IPackage<ChecksumPackage>.Size => Size;

    [FieldOffset(0)]
    public UInt128 Checksum;

    public void Serialize(ReadOnlySpan<byte> buffer)
    {
        int size;
        unsafe
        {
            size = sizeof(ChecksumPackage);
        }
        Assert(size == Size, "The package size should be expected");
        Assert(buffer.Length >= size, "Buffer must have space for message");
        ref var payload = ref Unsafe.As<ChecksumPackage, byte>(ref this);

        Unsafe.CopyBlock(ref MemoryMarshal.GetReference(buffer), ref payload, (uint)size);
    }

    public static bool TryParse(ReadOnlySpan<byte> buffer, ref PackageHeader header, out ChecksumPackage package)
    {
        int size;
        unsafe
        {
            size = sizeof(ChecksumPackage);
        }
        Assert(size == Size, "The package size should be expected");

        package = default;
        if (buffer.Length < Size)
            return false;

        Unsafe.CopyBlock(
            ref Unsafe.As<ChecksumPackage, byte>(ref package),
            ref MemoryMarshal.GetReference(buffer),
            (uint)size
        );

        return true;
    }
}

[StructLayout(LayoutKind.Explicit, Size = Size, Pack = 16)]
public struct SessionPackage : IPackage<SessionPackage>
{
    public const int Size = 32;

    static int IPackage<SessionPackage>.Size => Size;

    [FieldOffset(0)]
    public UInt128 Id;

    [FieldOffset(16)]
    public byte Partitions;

    public void Serialize(ReadOnlySpan<byte> buffer)
    {
        int size;
        unsafe
        {
            size = sizeof(SessionPackage);
        }
        Assert(size == Size, "The package size should be expected");
        Assert(buffer.Length >= size, "Buffer must have space for message");
        ref var payload = ref Unsafe.As<SessionPackage, byte>(ref this);

        Unsafe.CopyBlock(ref MemoryMarshal.GetReference(buffer), ref payload, (uint)size);
    }

    public static bool TryParse(ReadOnlySpan<byte> buffer, ref PackageHeader header, out SessionPackage package)
    {
        Assert((PackageType)header.PackageType == PackageType.Session, "Should be a session description");

        int size;
        unsafe
        {
            size = sizeof(SessionPackage);
        }
        Assert(size == Size, "The package size should be expected");

        package = default;
        if (buffer.Length < Size)
            return false;

        Unsafe.CopyBlock(
            ref Unsafe.As<SessionPackage, byte>(ref package),
            ref MemoryMarshal.GetReference(buffer),
            (uint)size
        );

        return true;
    }
}

[StructLayout(LayoutKind.Explicit, Size = Size, Pack = 8)]
public struct OkPackage : IPackage<OkPackage>
{
    private const int Size = 8;

    static int IPackage<OkPackage>.Size => Size;

    [FieldOffset(0)]
    public ulong ReplyFor;

    public void Serialize(ReadOnlySpan<byte> buffer)
    {
        int size;
        unsafe
        {
            size = sizeof(OkPackage);
        }
        Assert(size == Size, "The package size should be expected");
        Assert(buffer.Length >= size, "Buffer must have space for message");
        ref var payload = ref Unsafe.As<OkPackage, byte>(ref this);

        Unsafe.CopyBlock(ref MemoryMarshal.GetReference(buffer), ref payload, (uint)size);
    }

    public static bool TryParse(ReadOnlySpan<byte> buffer, ref PackageHeader header, out OkPackage package)
    {
        Assert((PackageType)header.PackageType == PackageType.Ok, "Should be corretly typed");

        int size;
        unsafe
        {
            size = sizeof(OkPackage);
        }
        Assert(size == Size, "The package size should be expected");

        package = default;
        if (buffer.Length < Size)
            return false;

        Unsafe.CopyBlock(
            ref Unsafe.As<OkPackage, byte>(ref package),
            ref MemoryMarshal.GetReference(buffer),
            (uint)size
        );

        return true;
    }
}
