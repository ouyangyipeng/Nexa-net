"""Transport Layer Compression Algorithm Comparison Chart."""
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

# Data from docs/PERFORMANCE.md (compression times, random data)
data_sizes = ['100B', '1KB', '10KB', '100KB']
data_sizes_num = [100, 1000, 10000, 100000]

lz4_compress = [387e-3, 814e-3, 2.03, 11.84]  # µs
zstd_compress = [20.70, 25.60, 31.77, 64.38]  # µs
gzip_compress = [16.26, 31.29, 177.85, 2920]   # µs (2.92ms = 2920µs)

lz4_decompress = [104e-3, 141e-3, 267e-3, 5.24]  # µs
zstd_decompress = [2.12, 2.02, 3.16, 15.59]  # µs
gzip_decompress = [3.74, 3.28, 4.78, 18.61]   # µs

fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

x = np.arange(len(data_sizes))
width = 0.25

# Left: Compression
bars1 = ax1.bar(x - width, lz4_compress, width, label='LZ4', color='#4CAF50', edgecolor='black', linewidth=0.5)
bars2 = ax1.bar(x, zstd_compress, width, label='Zstd', color='#2196F3', edgecolor='black', linewidth=0.5)
bars3 = ax1.bar(x + width, gzip_compress, width, label='Gzip', color='#F44336', edgecolor='black', linewidth=0.5)

ax1.set_xlabel('Data Size', fontsize=12)
ax1.set_ylabel('Compression Time (µs)', fontsize=12)
ax1.set_title('Compression Time by Algorithm & Data Size', fontsize=13, fontweight='bold')
ax1.set_xticks(x)
ax1.set_xticklabels(data_sizes)
ax1.legend(fontsize=11)
ax1.grid(axis='y', alpha=0.3, linestyle='--')

# Right: Decompression
bars4 = ax2.bar(x - width, lz4_decompress, width, label='LZ4', color='#4CAF50', edgecolor='black', linewidth=0.5)
bars5 = ax2.bar(x, zstd_decompress, width, label='Zstd', color='#2196F3', edgecolor='black', linewidth=0.5)
bars6 = ax2.bar(x + width, gzip_decompress, width, label='Gzip', color='#F44336', edgecolor='black', linewidth=0.5)

ax2.set_xlabel('Data Size', fontsize=12)
ax2.set_ylabel('Decompression Time (µs)', fontsize=12)
ax2.set_title('Decompression Time by Algorithm & Data Size', fontsize=13, fontweight='bold')
ax2.set_xticks(x)
ax2.set_xticklabels(data_sizes)
ax2.legend(fontsize=11)
ax2.grid(axis='y', alpha=0.3, linestyle='--')

plt.tight_layout()
plt.savefig('figures/transport_bench.pdf', bbox_inches='tight')
print("Generated figures/transport_bench.pdf")