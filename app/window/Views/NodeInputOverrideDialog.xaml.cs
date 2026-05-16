using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.Text.Json.Nodes;

namespace ManualWindow.Views;

public sealed partial class NodeInputOverrideDialog : ContentDialog
{
    public JsonObject? Result { get; private set; }

    public NodeInputOverrideDialog()
    {
        InitializeComponent();
        PrimaryButtonClick += ValidateAndCapture;
    }

    public void SetInitialJson(JsonObject? existing)
    {
        if (existing is not null)
            JsonInput.Text = existing.ToJsonString(
                new System.Text.Json.JsonSerializerOptions { WriteIndented = true });
    }

    private void ValidateAndCapture(ContentDialog sender, ContentDialogButtonClickEventArgs args)
    {
        var text = JsonInput.Text.Trim();
        if (string.IsNullOrEmpty(text)) { text = "{}"; }

        try
        {
            var node = JsonNode.Parse(text);
            if (node is not JsonObject obj)
            {
                ErrorText.Text = "Input must be a JSON object { ... }";
                ErrorText.Visibility = Visibility.Visible;
                args.Cancel = true;
                return;
            }
            Result = obj;
            ErrorText.Visibility = Visibility.Collapsed;
        }
        catch (System.Text.Json.JsonException ex)
        {
            ErrorText.Text = $"Invalid JSON: {ex.Message}";
            ErrorText.Visibility = Visibility.Visible;
            args.Cancel = true;
        }
    }
}
