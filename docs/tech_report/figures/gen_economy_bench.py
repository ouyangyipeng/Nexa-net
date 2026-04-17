"""Economy Layer Channel TPS and Receipt Operations Chart."""
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

# Data from docs/PERFORMANCE.md
ops_counts = [1, 10, 100, 1000]
channel_tps_ns = [100.86, 1030, 11050, 106740]  # convert to µs: divide by 1000
channel_tps_us = [v/1000 for v in channel_tps_ns]  # [0.10086, 1.03, 11.05, 106.74]

receipt_ops = ['Sign\n(Payer)', 'Sign\n(Both)', 'Verify\n(Both)', 'Hash\n(SHA-256)', 'Chain\n(10 receipts)']
receipt_times = [38.61, 77.70, 81.24, 0.39093, 17.33]  # µs

fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5))

# Left: Channel update TPS vs operation count
ax1.plot(ops_counts, channel_tps_us, 'D-', color='#2196F3', linewidth=2, markersize=8)
ax1.fill_between(ops_counts, channel_tps_us, alpha=0.15, color='#2196F3')
for x, y in zip(ops_counts, channel_tps_us):
    tps = 1e6 / (y * 1000)  # convert µs back to ns, then compute TPS
    ax1.annotate(f'{y:.2f} µs\n({tps/1e6:.1f}M TPS)', (x, y),
                 textcoords="offset points", xytext=(15, 10), ha='left', fontsize=9)
ax1.set_xlabel('Number of Transfer Operations', fontsize=12)
ax1.set_ylabel('Total Time (µs)', fontsize=12)
ax1.set_title('State Channel Update: Time vs Operations', fontsize=13, fontweight='bold')
ax1.set_xscale('log')
ax1.set_yscale('log')
ax1.grid(True, alpha=0.3, linestyle='--')

# Right: Receipt operation latency
colors_receipt = ['#FF9800', '#F44336', '#9C27B0', '#4CAF50', '#2196F3']
bars = ax2.bar(receipt_ops, receipt_times, color=colors_receipt, width=0.55,
               edgecolor='black', linewidth=0.5)
for bar, val in zip(bars, receipt_times):
    height = bar.get_height()
    if val < 1:
        label = f'{val*1000:.1f} ns'
    else:
        label = f'{val:.2f} µs'
    ax2.text(bar.get_x() + bar.get_width()/2., height + 2,
            label, ha='center', va='bottom', fontsize=10, fontweight='bold')
ax2.set_ylabel('Execution Time (µs)', fontsize=12)
ax2.set_title('Receipt Operations Latency', fontsize=13, fontweight='bold')
ax2.set_ylim(0, max(receipt_times) * 1.2)
ax2.grid(axis='y', alpha=0.3, linestyle='--')

plt.tight_layout()
plt.savefig('figures/economy_bench.pdf', bbox_inches='tight')
print("Generated figures/economy_bench.pdf")