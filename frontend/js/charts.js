class ChartManager {
    constructor() {
        this.charts = {};
        this.dataCache = {
            deviation: [],
            intensity: [],
            moment: [],
            temperature: []
        };
    }
    
    init() {
        this.createDeviationChart();
        this.createIntensityChart();
        this.createMomentChart();
        this.createTemperatureChart();
    }
    
    createDeviationChart() {
        const ctx = document.getElementById('deviationChart');
        if (!ctx) return;
        
        this.charts.deviation = new Chart(ctx, {
            type: 'line',
            data: {
                labels: [],
                datasets: [
                    {
                        label: '指向偏差 (°)',
                        data: [],
                        borderColor: '#64b5f6',
                        backgroundColor: 'rgba(100, 181, 246, 0.1)',
                        borderWidth: 2,
                        fill: true,
                        tension: 0.4
                    },
                    {
                        label: '告警阈值 (5°)',
                        data: [],
                        borderColor: '#ff9800',
                        borderWidth: 1,
                        borderDash: [5, 5],
                        fill: false,
                        pointRadius: 0
                    },
                    {
                        label: '严重阈值 (10°)',
                        data: [],
                        borderColor: '#ff5252',
                        borderWidth: 1,
                        borderDash: [5, 5],
                        fill: false,
                        pointRadius: 0
                    }
                ]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: {
                        labels: {
                            color: '#aaa',
                            font: { size: 11 }
                        }
                    }
                },
                scales: {
                    x: {
                        ticks: { color: '#888', maxTicksLimit: 8 },
                        grid: { color: 'rgba(100, 150, 255, 0.1)' }
                    },
                    y: {
                        min: 0,
                        max: 15,
                        ticks: { color: '#888' },
                        grid: { color: 'rgba(100, 150, 255, 0.1)' },
                        title: {
                            display: true,
                            text: '偏差角度 (°)',
                            color: '#64b5f6'
                        }
                    }
                },
                interaction: {
                    intersect: false,
                    mode: 'index'
                }
            }
        });
    }
    
    createIntensityChart() {
        const ctx = document.getElementById('intensityChart');
        if (!ctx) return;
        
        this.charts.intensity = new Chart(ctx, {
            type: 'bar',
            data: {
                labels: ['汉长安城', '洛阳故城', '马王堆', '未央宫', '狮子山'],
                datasets: [{
                    label: '地磁场强度 (nT)',
                    data: [55000, 54500, 52000, 54800, 53500],
                    backgroundColor: [
                        'rgba(100, 181, 246, 0.7)',
                        'rgba(124, 77, 255, 0.7)',
                        'rgba(0, 188, 212, 0.7)',
                        'rgba(76, 175, 80, 0.7)',
                        'rgba(255, 152, 0, 0.7)'
                    ],
                    borderColor: [
                        '#64b5f6',
                        '#7c4dff',
                        '#00bcd4',
                        '#4caf50',
                        '#ff9800'
                    ],
                    borderWidth: 1,
                    borderRadius: 4
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: { display: false }
                },
                scales: {
                    x: {
                        ticks: { color: '#888', font: { size: 10 } },
                        grid: { display: false }
                    },
                    y: {
                        min: 45000,
                        max: 60000,
                        ticks: { color: '#888' },
                        grid: { color: 'rgba(100, 150, 255, 0.1)' },
                        title: {
                            display: true,
                            text: '场强 (nT)',
                            color: '#64b5f6'
                        }
                    }
                }
            }
        });
    }
    
    createMomentChart() {
        const ctx = document.getElementById('momentChart');
        if (!ctx) return;
        
        this.charts.moment = new Chart(ctx, {
            type: 'line',
            data: {
                labels: [],
                datasets: [
                    {
                        label: '磁矩X分量',
                        data: [],
                        borderColor: '#ff6b6b',
                        backgroundColor: 'rgba(255, 107, 107, 0.1)',
                        borderWidth: 2,
                        tension: 0.4
                    },
                    {
                        label: '磁矩Y分量',
                        data: [],
                        borderColor: '#4ecdc4',
                        backgroundColor: 'rgba(78, 205, 196, 0.1)',
                        borderWidth: 2,
                        tension: 0.4
                    },
                    {
                        label: '磁矩Z分量',
                        data: [],
                        borderColor: '#ffe66d',
                        backgroundColor: 'rgba(255, 230, 109, 0.1)',
                        borderWidth: 2,
                        tension: 0.4
                    }
                ]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: {
                        labels: {
                            color: '#aaa',
                            font: { size: 11 }
                        }
                    }
                },
                scales: {
                    x: {
                        ticks: { color: '#888', maxTicksLimit: 8 },
                        grid: { color: 'rgba(100, 150, 255, 0.1)' }
                    },
                    y: {
                        ticks: { color: '#888' },
                        grid: { color: 'rgba(100, 150, 255, 0.1)' },
                        title: {
                            display: true,
                            text: '磁矩分量 (A·m²)',
                            color: '#64b5f6'
                        }
                    }
                }
            }
        });
    }
    
    createTemperatureChart() {
        const ctx = document.getElementById('tempChart');
        if (!ctx) return;
        
        this.charts.temperature = new Chart(ctx, {
            type: 'scatter',
            data: {
                datasets: [{
                    label: '温度 vs 偏差',
                    data: [],
                    backgroundColor: 'rgba(100, 181, 246, 0.6)',
                    borderColor: '#64b5f6',
                    borderWidth: 1,
                    pointRadius: 6,
                    pointHoverRadius: 8
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: { display: false },
                    tooltip: {
                        callbacks: {
                            label: function(context) {
                                return `温度: ${context.parsed.x}°C, 偏差: ${context.parsed.y}°`;
                            }
                        }
                    }
                },
                scales: {
                    x: {
                        type: 'linear',
                        position: 'bottom',
                        ticks: { color: '#888' },
                        grid: { color: 'rgba(100, 150, 255, 0.1)' },
                        title: {
                            display: true,
                            text: '环境温度 (°C)',
                            color: '#64b5f6'
                        }
                    },
                    y: {
                        ticks: { color: '#888' },
                        grid: { color: 'rgba(100, 150, 255, 0.1)' },
                        title: {
                            display: true,
                            text: '指向偏差 (°)',
                            color: '#64b5f6'
                        }
                    }
                }
            }
        });
    }
    
    updateDeviationData(dataArray) {
        if (!this.charts.deviation) return;
        
        const labels = dataArray.map(d => {
            const date = new Date(d.timestamp);
            return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
        });
        
        const deviations = dataArray.map(d => d.pointing_deviation);
        const warningThreshold = Array(deviations.length).fill(CONFIG.THRESHOLDS.WARNING);
        const criticalThreshold = Array(deviations.length).fill(CONFIG.THRESHOLDS.CRITICAL);
        
        this.charts.deviation.data.labels = labels.slice(-20);
        this.charts.deviation.data.datasets[0].data = deviations.slice(-20);
        this.charts.deviation.data.datasets[1].data = warningThreshold.slice(-20);
        this.charts.deviation.data.datasets[2].data = criticalThreshold.slice(-20);
        this.charts.deviation.update('none');
    }
    
    updateMomentData(dataArray) {
        if (!this.charts.moment) return;
        
        const labels = dataArray.map(d => {
            const date = new Date(d.timestamp);
            return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
        });
        
        const mx = dataArray.map(d => d.magnetic_moment_x);
        const my = dataArray.map(d => d.magnetic_moment_y);
        const mz = dataArray.map(d => d.magnetic_moment_z);
        
        this.charts.moment.data.labels = labels.slice(-20);
        this.charts.moment.data.datasets[0].data = mx.slice(-20);
        this.charts.moment.data.datasets[1].data = my.slice(-20);
        this.charts.moment.data.datasets[2].data = mz.slice(-20);
        this.charts.moment.update('none');
    }
    
    updateTemperatureData(dataArray) {
        if (!this.charts.temperature) return;
        
        const scatterData = dataArray.map(d => ({
            x: d.environment_temp,
            y: d.pointing_deviation
        }));
        
        this.charts.temperature.data.datasets[0].data = scatterData.slice(-50);
        this.charts.temperature.update('none');
    }
    
    addSensorDataPoint(data) {
        this.dataCache.deviation.push(data);
        if (this.dataCache.deviation.length > 100) {
            this.dataCache.deviation.shift();
        }
        
        this.updateDeviationData(this.dataCache.deviation);
        this.updateMomentData(this.dataCache.deviation);
        this.updateTemperatureData(this.dataCache.deviation);
    }
    
    updateIntensityData(siteData) {
        if (!this.charts.intensity) return;
        
        this.charts.intensity.data.labels = siteData.map(s => s.site_name);
        this.charts.intensity.data.datasets[0].data = siteData.map(s => s.intensity);
        this.charts.intensity.update();
    }
    
    destroy() {
        Object.values(this.charts).forEach(chart => {
            if (chart) chart.destroy();
        });
        this.charts = {};
    }
}
