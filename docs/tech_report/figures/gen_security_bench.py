"""Security Layer AES-GCM Latency vs Data Size Chart."""
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

# Data from docs/PERFORMANCE.md
data_sizes_bytes = [16, 256, 1024, 4096, 16384]
data_labels = ['16B', '256B', '1KB', '4KB', '16KB']
latency_us = [1.30, 1.98, 5.11, 16.40, 61.99]

fig, ax = plt.subplots(figsize=(10, 6))

ax.plot(data_sizes_bytes, latency_us, 'o-', color='#F44336', linewidth=2.5, markersize=10)
ax.fill_between(data_sizes_bytes, latency_us, alpha=0.15, color='#F44336')

for x, y, label in zip(data_sizes_bytes, latency_us, data_labels):
    ax.annotate(f'{label}: {y:.2f} µs', (x, y), textcoords="offset points",
                xytext=(20, -5), ha='left', fontsize=10, fontweight='bold',
                arrowprops=dict(arrowstyle='->', color='gray', lw=0.8))

# Add linear trend line
coeffs = np.polyfit(data_sizes_bytes, latency_us, 1)
trend_x = np.linspace(0, max(data_sizes_bytes), 100)
trend_y = np.polyval(coeffs, trend_x)
ax.plot(trend_x, trend_y, '--', color='gray', alpha=0.5, linewidth=1,
        label=f'Linear fit: ~{coeffs[0]*1000:.2f} µs/KB')

ax.set_xlabel('Data Size (bytes)', fontsize=12)
ax.set_ylabel('Encrypt+Decrypt Time (µs)', fontsize=12)
ax.set_title('AES-256-GCM Latency vs Data Size', fontsize=14, fontweight='bold')
ax.set_xscale('log')
ax.legend(fontsize=10)
ax.grid(True, alpha=0.3, linestyle='--')

plt.tight_layout()
plt.savefig('figures/security_bench.pdf', bbox_inches='tight')
print("Generated figures/security_bench.pdf")