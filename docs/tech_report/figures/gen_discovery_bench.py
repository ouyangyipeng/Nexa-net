"""Discovery Layer HNSW Performance Charts."""
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

# Data from docs/PERFORMANCE.md
index_sizes = [100, 1000, 10000]
search_times_us = [25.52, 34.65, 45.64]  # µs
insert_times_ms = [10.83, 83.40, 726.60]  # ms

fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5))

# Left: Search latency (sub-linear scaling)
ax1.plot(index_sizes, search_times_us, 'o-', color='#2196F3', linewidth=2, markersize=8)
ax1.fill_between(index_sizes, search_times_us, alpha=0.15, color='#2196F3')
for x, y in zip(index_sizes, search_times_us):
    ax1.annotate(f'{y:.2f} µs', (x, y), textcoords="offset points",
                 xytext=(0, 12), ha='center', fontsize=9, fontweight='bold')
ax1.set_xlabel('Index Size (vectors)', fontsize=12)
ax1.set_ylabel('Search Latency (µs)', fontsize=12)
ax1.set_title('HNSW Search Latency vs Index Size', fontsize=13, fontweight='bold')
ax1.set_xscale('log')
ax1.grid(True, alpha=0.3, linestyle='--')

# Right: Insert time (O(n·log n) scaling)
ax2.plot(index_sizes, insert_times_ms, 's-', color='#F44336', linewidth=2, markersize=8)
ax2.fill_between(index_sizes, insert_times_ms, alpha=0.15, color='#F44336')
for x, y in zip(index_sizes, insert_times_ms):
    ax2.annotate(f'{y:.1f} ms', (x, y), textcoords="offset points",
                 xytext=(0, 12), ha='center', fontsize=9, fontweight='bold')
ax2.set_xlabel('Index Size (vectors)', fontsize=12)
ax2.set_ylabel('Insert Time (ms)', fontsize=12)
ax2.set_title('HNSW Insert Time vs Index Size', fontsize=13, fontweight='bold')
ax2.set_xscale('log')
ax2.set_yscale('log')
ax2.grid(True, alpha=0.3, linestyle='--')

plt.tight_layout()
plt.savefig('figures/discovery_bench.pdf', bbox_inches='tight')
print("Generated figures/discovery_bench.pdf")