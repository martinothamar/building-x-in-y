using Sync.Core;

namespace Sync.Test;

public class ProtocolTests
{
    [Fact]
    public void Session_Package_Serialization_Roundtrip()
    {
        ulong msgId = 1337;
        UInt128 sessionId = 7331;
        byte workerCount = 8;
        ulong partitionId = ulong.MaxValue;

        // Serialize the package and compute the checksum manually
        byte[] buffer = new byte[Protocol.SizeFor<SessionPackage>()];
        var header = new PackageHeader
        {
            PartitionId = partitionId,
            MessageId = msgId,
            PackageType = (byte)PackageType.Session,
        };
        var package = new SessionPackage { Id = sessionId, Partitions = workerCount, };
        Protocol.Serialize(buffer, ref header, ref package);
        var checksum = Checksummer.Compute(buffer.AsSpan(0, PackageHeader.Size + SessionPackage.Size));

        // Parse and verify the header
        Assert.True(Protocol.TryParseHeader(buffer, out var deserializedHeader));
        Assert.Equal(header.PartitionId, deserializedHeader.PartitionId);
        Assert.Equal(header.MessageId, deserializedHeader.MessageId);
        Assert.Equal(header.PackageType, deserializedHeader.PackageType);

        // Parse and verify the package
        Assert.True(
            Protocol.TryParsePackage<SessionPackage>(
                buffer,
                ref header,
                out var deserializedPackage,
                out var deserializedChecksum
            )
        );
        Assert.Equal(package.Id, deserializedPackage.Id);
        Assert.Equal(package.Partitions, deserializedPackage.Partitions);
        // Verify the checksum as well
        Assert.Equal(checksum, deserializedChecksum);
    }
}
