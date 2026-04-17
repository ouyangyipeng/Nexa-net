"""REST API Endpoint Latency Comparison Chart."""
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

# Data from docs/PERFORMANCE.md
endpoints = ['/v1/health', '/v1/register', '/v1/discover']
mean_times = [1.202, 1.270, 1.449]  # ms
median_times = [1.114, 1.175, 1.266]  # ms
business_logic = [0.0, 0.3, 0.5]  # ms (estimated from docs)
http_overhead = [1.202, 0.97, 0.949]  # ms (approximate)

fig, ax = plt.subplots(figsize=(10, 6))

x = np.arange(len(endpoints))
width = 0.3

bars1 = ax.bar(x - width, mean_times, width, label='Mean Latency', color='#2196F3',
               edgecolor='black', linewidth=0.5)
bars2 = ax.bar(x, median_times, width, label='Median Latency', color='#4CAF50',
               edgecolor='black', linewidth=0.5)
bars3 = ax.bar(x + width, business_logic, width, label='Business Logic (est.)', color='#FF9800',
               edgecolor='black', linewidth=0.5)

# Add value labels
for bars, values in [(bars1, mean_times), (bars2, median_times), (bars3, business_logic)]:
    for bar, val in zip(bars, values):
        height = bar.get_height()
        if val > 0:
            ax.text(bar.get_x() + bar.get_width()/2., height + 0.02,
                    f'{val:.2f}ms', ha='center', va='bottom', fontsize=9, fontweight='bold')

ax.set_xlabel('API Endpoint', fontsize=12)
ax.set_ylabel('Latency (ms)', fontsize=12)
ax.set_title('REST API Endpoint Latency Comparison', fontsize=14, fontweight='bold')
ax.set_xticks(x)
ax.set_xticklabels(endpoints, fontsize=11)
ax.legend(fontsize=11)
ax.set_ylim(0, max(mean_times) * 1.25)
ax.grid(axis='y', alpha=0.3, linestyle='--')

plt.tight_layout()
plt.savefig('figures/api_bench.pdf', bbox_inches='tight')
print("Generated figures/api_bench.pdf")