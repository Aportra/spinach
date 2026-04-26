import io
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches
from matplotlib.colors import LinearSegmentedColormap


def chart(df, config):

    fig, ax = plt.subplots(figsize=(12, 6))
    chart_type = config["chart_type"]
    x = config["x"]
    y = config["y"]
    title = config.get("title", "")

    if chart_type == "line":
        ax.plot(df[x], df[y], color='#5865F2', linewidth=2, marker='o')
    elif chart_type == "bar":
        ax.bar(df[x], df[y], color='#5865F2')
    elif chart_type == "scatter":
        ax.scatter(df[x], df[y], color='#5865F2')

    ax.set_title(title, color='white', fontsize=14, fontweight='bold')
    ax.set_facecolor('#2C2F33')
    fig.patch.set_facecolor('#23272A')
    ax.tick_params(colors='#DCDDDE')
    ax.xaxis.label.set_color('#DCDDDE')
    ax.yaxis.label.set_color('#DCDDDE')
    for spine in ax.spines.values():
        spine.set_edgecolor('#40444B')

    buf = io.BytesIO()
    plt.savefig(buf, format='png', bbox_inches='tight', dpi=150)
    buf.seek(0)
    plt.close(fig)
    return buf

def table(results):
        fig, ax = plt.subplots(figsize=(14, len(results) * 0.6 + 2))
        ax.axis('off')

        table = ax.table(
            cellText=results.values,
            colLabels=results.columns,
            loc='center',
            cellLoc='center'
        )

        table.auto_set_font_size(False)
        table.set_fontsize(10)
        table.auto_set_column_width(col=list(range(len(results.columns))))

# Header styling
        for col in range(len(results.columns)):
            cell = table[0, col]
            cell.set_facecolor('#5865F2')  # Discord blurple
            cell.set_text_props(color='white', fontweight='bold', fontsize=11)
            cell.set_edgecolor('#23272A')
            cell.set_height(0.08)

# Row styling
        for row in range(1, len(results) + 1):
            for col in range(len(results.columns)):
                cell = table[row, col]
                cell.set_facecolor('#2C2F33' if row % 2 == 0 else '#36393F')
                cell.set_text_props(color='#DCDDDE')
                cell.set_edgecolor('#23272A')

# Title
        fig.text(0.5, 0.97, 'Query Results', ha='center', va='top',
                 fontsize=14, fontweight='bold', color='white')

# Subtle accent line under title
        fig.add_artist(mpatches.FancyArrowPatch(
            (0.1, 0.93), (0.9, 0.93),
            arrowstyle='-', color='#5865F2',
            linewidth=2, transform=fig.transFigure
        ))

        fig.patch.set_facecolor('#23272A')

        buf = io.BytesIO()
        plt.savefig(buf, format='png', bbox_inches='tight', dpi=100)
        buf.seek(0)
        plt.close(fig)

        return buf
