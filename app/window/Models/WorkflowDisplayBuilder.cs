using System.Text.Json.Nodes;

namespace ManualWindow.Models;

public static class WorkflowDisplayBuilder
{
    public static WorkflowDisplayModel Build(JsonObject workflow)
    {
        var workflowId = workflow["id"]?.GetValue<string>() ?? "untitled-workflow";
        var nodeArray = workflow["nodes"]?.AsArray() ?? new JsonArray();
        var depArray = workflow["dependencies"]?.AsArray() ?? new JsonArray();

        var nodeObjects = nodeArray
            .Select(n => n?.AsObject())
            .Where(n => n is not null)
            .Select(n => n!)
            .ToList();

        var depObjects = depArray
            .Select(d => d?.AsObject())
            .Where(d => d is not null)
            .Select(d => d!)
            .ToList();

        var nodeIds = nodeObjects
            .Select(o => o["id"]?.GetValue<string>())
            .Where(id => id is not null)
            .Select(id => id!)
            .ToList();

        var stages = LayoutStages(nodeIds, depObjects);
        var positions = PositionsByNodeId(stages);

        var nodes = nodeObjects
            .Select((obj, offset) =>
            {
                var id = obj["id"]?.GetValue<string>();
                if (id is null) return null;
                var serverKind = obj["kind"]?.GetValue<string>() ?? "template";
                return new WorkflowNodeModel(
                    id: id,
                    title: TitleFor(id),
                    subtitle: SubtitleFor(obj),
                    kind: WorkflowNodeKindExtensions.FromServerKind(serverKind),
                    position: positions.TryGetValue(id, out var pos)
                        ? pos
                        : FallbackPosition(offset, Math.Max(nodeObjects.Count, 1))
                );
            })
            .Where(n => n is not null)
            .Select(n => n!)
            .ToList();

        var edges = depObjects
            .Select(dep =>
            {
                var from = dep["depends_on"]?.GetValue<string>();
                var to = dep["node"]?.GetValue<string>();
                if (from is null || to is null) return null;
                return new WorkflowEdgeModel(from, to);
            })
            .Where(e => e is not null)
            .Select(e => e!)
            .ToList();

        return new WorkflowDisplayModel(workflowId, nodes, edges);
    }

    private static List<List<string>> LayoutStages(
        List<string> nodeIds,
        List<JsonObject> dependencies)
    {
        var remaining = new HashSet<string>(nodeIds);
        var completed = new HashSet<string>();
        var stages = new List<List<string>>();

        while (remaining.Count > 0)
        {
            var ready = remaining
                .Where(nodeId =>
                    dependencies
                        .Where(d => d["node"]?.GetValue<string>() == nodeId)
                        .All(dep =>
                        {
                            var upstream = dep["depends_on"]?.GetValue<string>();
                            return upstream is null || completed.Contains(upstream);
                        }))
                .OrderBy(id => id)
                .ToList();

            var stage = ready.Count == 0 ? remaining.OrderBy(id => id).ToList() : ready;
            stages.Add(stage);
            foreach (var id in stage) completed.Add(id);
            foreach (var id in stage) remaining.Remove(id);
        }

        return stages;
    }

    private static Dictionary<string, WorkflowNodePosition> PositionsByNodeId(
        List<List<string>> stages)
    {
        var positions = new Dictionary<string, WorkflowNodePosition>();
        var stageCount = Math.Max(stages.Count, 1);

        for (var stageIndex = 0; stageIndex < stages.Count; stageIndex++)
        {
            var stage = stages[stageIndex];
            var x = stageCount == 1
                ? 0.50
                : 0.12 + (0.76 * stageIndex / (double)(stageCount - 1));
            var rowCount = Math.Max(stage.Count, 1);

            for (var rowIndex = 0; rowIndex < stage.Count; rowIndex++)
            {
                var y = rowCount == 1
                    ? 0.50
                    : 0.24 + (0.52 * rowIndex / (double)(rowCount - 1));
                positions[stage[rowIndex]] = new WorkflowNodePosition(x, y);
            }
        }

        return positions;
    }

    private static WorkflowNodePosition FallbackPosition(int offset, int count) =>
        new(
            X: 0.15 + (0.70 * offset / (double)Math.Max(count - 1, 1)),
            Y: 0.50
        );

    private static string TitleFor(string id) =>
        string.Join(" ", id.Split('_')
            .Select(word => word.Length == 0
                ? word
                : char.ToUpperInvariant(word[0]) + word[1..]));

    private static string SubtitleFor(JsonObject obj)
    {
        var kind = obj["kind"]?.GetValue<string>() ?? "template";
        return kind switch
        {
            "constant" => "Constant payload",
            "delay"    => $"{(obj["duration_ms"]?.GetValue<int>() ?? 0)} ms delay",
            "fail"     => obj["error"]?.GetValue<string>() ?? "Failure node",
            "pi"       => obj["model"]?.GetValue<string>() ?? "Pi agent",
            "claude"   => obj["model"]?.GetValue<string>() ?? "Claude reviewer",
            "codex"    => obj["model"]?.GetValue<string>() ?? "Codex reviewer",
            "template" => obj["template"]?.GetValue<string>() ?? "Template",
            _          => kind,
        };
    }
}
