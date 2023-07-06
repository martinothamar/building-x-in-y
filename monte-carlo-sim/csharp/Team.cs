using System.Text.Json.Serialization;

internal readonly record struct TeamDto(
    [property: JsonPropertyName("name")] string Name,
    [property: JsonPropertyName("expectedGoals")] double ExpectedGoals
);

[JsonSerializable(typeof(TeamDto[]))]
internal partial class AppJsonSerializerContext : JsonSerializerContext { }
