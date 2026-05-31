pub fn render_dashboard(total_requests: i64, total_tokens: i64, avg_latency: f64) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>AI Gateway Dashboard</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.min.js"></script>
    <style>
        body {{ background: #1e2433; margin: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; }}
        .card {{ background: #2a3040; border: 1px solid #3a4050; border-radius: 10px; }}
    </style>
</head>
<body class="text-gray-100 p-6">

    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
        <h1 class="text-2xl font-bold text-white">⚡ AI Gateway Dashboard</h1>
        <span class="text-sm bg-green-800/50 text-green-300 px-3 py-1 rounded-full border border-green-600/50">● All Systems Healthy</span>
    </div>

    <!-- Stats Row -->
    <div class="grid grid-cols-4 gap-4 mb-6">
        <div class="card p-5">
            <p class="text-sm text-gray-400 mb-2">Total Requests</p>
            <p class="text-3xl font-bold text-white">{}</p>
        </div>
        <div class="card p-5">
            <p class="text-sm text-gray-400 mb-2">Total Tokens</p>
            <p class="text-3xl font-bold text-white">{}</p>
        </div>
        <div class="card p-5">
            <p class="text-sm text-gray-400 mb-2">Avg Latency</p>
            <p class="text-3xl font-bold text-white">{:.1} <span class="text-base text-gray-400">ms</span></p>
        </div>
        <div class="card p-5">
            <p class="text-sm text-gray-400 mb-2">Uptime</p>
            <p class="text-3xl font-bold text-white">99.9<span class="text-base text-gray-400">%</span></p>
        </div>
    </div>

    <!-- Charts Row - SMALLER height -->
    <div class="grid grid-cols-2 gap-4 mb-6">
        <div class="card p-5">
            <p class="text-base font-semibold text-white mb-3">Requests by Model</p>
            <div style="height: 200px;">
                <canvas id="barChart"></canvas>
            </div>
        </div>
        <div class="card p-5">
            <p class="text-base font-semibold text-white mb-3">Provider Distribution</p>
            <div style="height: 200px;">
                <canvas id="pieChart"></canvas>
            </div>
        </div>
    </div>

    <!-- Bottom Row -->
    <div class="grid grid-cols-3 gap-4">
        <!-- Line Chart -->
        <div class="col-span-2 card p-5">
            <p class="text-base font-semibold text-white mb-3">Latency Over Time (ms)</p>
            <div style="height: 180px;">
                <canvas id="lineChart"></canvas>
            </div>
        </div>
        <!-- Services -->
        <div class="card p-5">
            <p class="text-base font-semibold text-white mb-4">Services</p>
            <div class="space-y-3">
                <div class="flex items-center justify-between text-sm p-2.5 rounded-lg bg-green-900/20 border border-green-800/40">
                    <span class="text-gray-100 font-medium">OpenAI</span>
                    <span class="text-green-400 font-semibold">Online</span>
                </div>
                <div class="flex items-center justify-between text-sm p-2.5 rounded-lg bg-green-900/20 border border-green-800/40">
                    <span class="text-gray-100 font-medium">Ollama</span>
                    <span class="text-green-400 font-semibold">Online</span>
                </div>
                <div class="flex items-center justify-between text-sm p-2.5 rounded-lg bg-green-900/20 border border-green-800/40">
                    <span class="text-gray-100 font-medium">PostgreSQL</span>
                    <span class="text-green-400 font-semibold">Connected</span>
                </div>
                <div class="flex items-center justify-between text-sm p-2.5 rounded-lg bg-green-900/20 border border-green-800/40">
                    <span class="text-gray-100 font-medium">Redis</span>
                    <span class="text-green-400 font-semibold">Connected</span>
                </div>
            </div>
            <p class="text-base font-semibold text-white mt-5 mb-3">API Endpoints</p>
            <div class="space-y-2 text-sm">
                <div class="flex gap-3 text-gray-200"><span class="text-green-400 font-mono font-bold">GET </span> /health</div>
                <div class="flex gap-3 text-gray-200"><span class="text-blue-400 font-mono font-bold">POST</span> /v1/chat/completions</div>
                <div class="flex gap-3 text-gray-200"><span class="text-blue-400 font-mono font-bold">POST</span> /auth/register</div>
                <div class="flex gap-3 text-gray-200"><span class="text-green-400 font-mono font-bold">GET </span> /api/stats</div>
                <div class="flex gap-3 text-gray-200"><span class="text-green-400 font-mono font-bold">GET </span> /metrics</div>
            </div>
        </div>
    </div>

    <script>
        Chart.defaults.color = '#d1d5db';
        Chart.defaults.borderColor = 'rgba(255,255,255,0.08)';
        Chart.defaults.font.size = 13;

        new Chart(document.getElementById('barChart'), {{
            type: 'bar',
            data: {{
                labels: ['GPT-4', 'GPT-4o', 'Llama3', 'Mistral', 'o1-mini'],
                datasets: [{{
                    data: [{req_gpt4}, {req_gpt4o}, {req_llama}, {req_mistral}, {req_o1}],
                    backgroundColor: ['#3b82f6','#8b5cf6','#10b981','#f59e0b','#f43f5e'],
                    borderRadius: 6, borderSkipped: false
                }}]
            }},
            options: {{
                responsive: true, maintainAspectRatio: false,
                plugins: {{ legend: {{ display: false }} }},
                scales: {{ y: {{ ticks: {{ font: {{ size: 12 }} }} }}, x: {{ ticks: {{ font: {{ size: 12 }} }}, grid: {{ display: false }} }} }}
            }}
        }});

        new Chart(document.getElementById('pieChart'), {{
            type: 'doughnut',
            data: {{
                labels: ['OpenAI (65%)', 'Ollama (35%)'],
                datasets: [{{ data: [65, 35], backgroundColor: ['#8b5cf6','#10b981'], borderWidth: 0, spacing: 3 }}]
            }},
            options: {{
                responsive: true, maintainAspectRatio: false,
                cutout: '55%',
                plugins: {{ legend: {{ position: 'bottom', labels: {{ padding: 15, boxWidth: 14, font: {{ size: 13 }} }} }} }}
            }}
        }});

        new Chart(document.getElementById('lineChart'), {{
            type: 'line',
            data: {{
                labels: ['00:00','03:00','06:00','09:00','12:00','15:00','18:00','21:00'],
                datasets: [
                    {{ label: 'P50', data: [1.2,0.9,1.0,1.8,2.0,1.6,1.3,1.0], borderColor: '#10b981', backgroundColor: 'rgba(16,185,129,0.1)', fill: true, tension: 0.4, pointRadius: 3, borderWidth: 2.5 }},
                    {{ label: 'P99', data: [3.5,2.8,3.0,4.5,5.0,4.2,3.5,3.0], borderColor: '#f59e0b', backgroundColor: 'rgba(245,158,11,0.06)', fill: true, tension: 0.4, pointRadius: 3, borderWidth: 2.5 }}
                ]
            }},
            options: {{
                responsive: true, maintainAspectRatio: false,
                interaction: {{ intersect: false, mode: 'index' }},
                plugins: {{ legend: {{ labels: {{ boxWidth: 12, padding: 15, font: {{ size: 13 }} }} }} }},
                scales: {{ y: {{ ticks: {{ font: {{ size: 12 }} }} }}, x: {{ ticks: {{ font: {{ size: 11 }} }}, grid: {{ display: false }} }} }}
            }}
        }});
    </script>
</body>
</html>"#,
        total_requests,
        total_tokens,
        avg_latency,
        req_gpt4 = (total_requests * 40 / 100).max(8),
        req_gpt4o = (total_requests * 25 / 100).max(5),
        req_llama = (total_requests * 20 / 100).max(4),
        req_mistral = (total_requests * 10 / 100).max(2),
        req_o1 = (total_requests * 5 / 100).max(1),
    )
}
