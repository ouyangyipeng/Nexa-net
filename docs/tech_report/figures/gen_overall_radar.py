"""Performance Targets vs Actual Results Radar Chart."""
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

# Performance dimensions and data
categories = [
    'Routing\nLatency',
    'Channel\nTPS',
    'Serialization\nThroughput',
    'Compression\nRatio',
    'API\nLatency',
]

# Normalize to 0-10 scale where 10 = "exceeds target significantly"
# Routing: target 100ms, actual 31µs = 3200x better → ~10
# Channel TPS: target 10K, actual 9.9M = 990x → ~9
# Serialization: target 100K ops/s, actual 5.6M = 56x → ~7
# Compression ratio: target 50%, actual 93% → ~9
# API latency: target <5ms, actual 1.4ms → ~8

target_scores = [5, 5, 5, 5, 5]  # Target = baseline (5)
actual_scores = [10, 9, 7, 9, 8]  # Actual performance scores

N = len(categories)
angles = np.linspace(0, 2 * np.pi, N, endpoint=False).tolist()

# Complete the loop
target_scores_loop = target_scores + target_scores[:1]
actual_scores_loop = actual_scores + actual_scores[:1]
angles_loop = angles + angles[:1]

fig, ax = plt.subplots(figsize=(8, 8), subplot_kw=dict(polar=True))

ax.plot(angles_loop, target_scores_loop, 'o-', linewidth=2, color='#9E9E9E',
        label='Target (baseline)', markersize=6)
ax.fill(angles_loop, target_scores_loop, alpha=0.1, color='#9E9E9E')

ax.plot(angles_loop, actual_scores_loop, 's-', linewidth=2.5, color='#2196F3',
        label='Actual Performance', markersize=8)
ax.fill(angles_loop, actual_scores_loop, alpha=0.25, color='#2196F3')

ax.set_xticks(angles)
ax.set_xticklabels(categories, fontsize=11)
ax.set_ylim(0, 11)
ax.set_yticks([2, 4, 6, 8, 10])
ax.set_yticklabels(['2', '4', '6', '8', '10'], fontsize=9)
ax.set_title('Performance: Target vs Actual', fontsize=14, fontweight='bold', pad=20)
ax.legend(loc='upper right', bbox_to_anchor=(1.3, 1.1), fontsize=11)

# Add annotation for key achievements
annotations = [
    '3200× better',
    '990× better',
    '56× better',
    '93% ratio',
    '<1.5ms',
]
for angle, score, note in zip(angles, actual_scores, annotations):
    ax.annotate(note, (angle, score + 0.8), fontsize=9, ha='center',
                color='#1565C0', fontweight='bold')

plt.tight_layout()
plt.savefig('figures/overall_radar.pdf', bbox_inches='tight')
print("Generated figures/overall_radar.pdf")