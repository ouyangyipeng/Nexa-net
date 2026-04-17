"""Test Coverage Distribution Pie Chart."""
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

# Data: 433 unit + 31 integration + 5 HTTP E2E + 10 e2e + 6 other = 485 total
categories = ['Unit Tests', 'Integration Tests', 'E2E Tests', 'HTTP E2E Tests', 'Other (proptest/etc.)']
counts = [433, 31, 10, 5, 6]
total = sum(counts)

colors = ['#2196F3', '#4CAF50', '#FF9800', '#9C27B0', '#795548']
explode = (0.05, 0.05, 0.05, 0.05, 0.05)

fig, ax = plt.subplots(figsize=(10, 7))

wedges, texts, autotexts = ax.pie(
    counts, explode=explode, labels=categories, colors=colors,
    autopct=lambda pct: f'{pct:.1f}%\n({int(round(pct/100.*total))})',
    shadow=False, startangle=90, textprops={'fontsize': 11},
    pctdistance=0.75,
)

for autotext in autotexts:
    autotext.set_fontweight('bold')
    autotext.set_fontsize(10)

ax.set_title(f'Test Coverage Distribution (Total: {total} tests)', fontsize=14, fontweight='bold')

# Add a summary table as text
summary_text = (
    f"Unit Tests: {counts[0]} ({counts[0]/total*100:.1f}%)\n"
    f"Integration: {counts[1]} ({counts[1]/total*100:.1f}%)\n"
    f"E2E Scenarios: {counts[2]}\n"
    f"HTTP E2E: {counts[3]}\n"
    f"Property-based: {counts[4]}\n"
    f"─────────────────\n"
    f"Total: {total} tests"
)

plt.tight_layout()
plt.savefig('figures/test_coverage.pdf', bbox_inches='tight')
print("Generated figures/test_coverage.pdf")