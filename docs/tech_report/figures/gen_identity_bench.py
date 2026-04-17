"""Identity Layer Performance Benchmark Chart."""
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

# Data from docs/PERFORMANCE.md
operations = [
    'Keypair\nGeneration',
    'DID\nParse',
    'Sign\nMessage',
    'Verify\nSignature',
    'Identity Keys\nGenerate',
    'Keystore\nStore',
    'Keystore\nStore+Get',
]
times_us = [18.71, 0.00975, 34.68, 38.54, 34.98, 0.276, 0.444]

# Convert ns to µs for very small values
times_us[1] = 9.75 / 1000  # 9.75 ns -> 0.00975 µs
times_us[5] = 276 / 1000   # 276 ns -> 0.276 µs
times_us[6] = 444 / 1000   # 444 ns -> 0.444 µs

fig, ax = plt.subplots(figsize=(10, 6))

colors = ['#2196F3', '#4CAF50', '#FF9800', '#F44336', '#9C27B0', '#00BCD4', '#795548']
bars = ax.bar(operations, times_us, color=colors, width=0.6, edgecolor='black', linewidth=0.5)

# Add value labels on bars
for bar, val in zip(bars, times_us):
    height = bar.get_height()
    if val < 1:
        label = f'{val*1000:.1f} ns'
    else:
        label = f'{val:.2f} µs'
    ax.text(bar.get_x() + bar.get_width()/2., height + max(times_us)*0.02,
            label, ha='center', va='bottom', fontsize=10, fontweight='bold')

ax.set_ylabel('Execution Time (µs)', fontsize=12)
ax.set_title('Identity Layer Performance Benchmarks', fontsize=14, fontweight='bold')
ax.set_ylim(0, max(times_us) * 1.15)
ax.grid(axis='y', alpha=0.3, linestyle='--')

plt.tight_layout()
plt.savefig('figures/identity_bench.pdf', bbox_inches='tight')
print("Generated figures/identity_bench.pdf")