let magneticPanel = null;

document.addEventListener('DOMContentLoaded', () => {
    initApp();
});

async function initApp() {
    try {
        await dataService.getHealth();
        showToast('后端服务连接成功', 'success');
    } catch (e) {
        showToast('无法连接后端服务，请检查服务是否启动', 'error');
    }

    window.sinan3d = new Sinan3D('sinanCanvas');

    magneticPanel = new MagneticPanel();
    magneticPanel.init();

    setupTabNavigation();
    startDataUpdates();
    startSensorStream();
}

function setupTabNavigation() {
    const tabButtons = document.querySelectorAll('.tab-btn');

    tabButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            const targetView = btn.dataset.view;

            tabButtons.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');

            document.querySelectorAll('.view').forEach(view => {
                view.classList.remove('active');
            });
            document.getElementById(targetView + 'View').classList.add('active');

            if (targetView === 'vectorfield') {
                setTimeout(() => magneticPanel.resizeVectorField(), 100);
            } else if (targetView === 'charts') {
                magneticPanel.resizeCharts();
            }
        });
    });
}

async function loadStatistics() {
    try {
        const response = await dataService.getStatistics();
        const stats = response.data || {};

        document.getElementById('deviceCount').textContent = stats.total_sensor_records ? '在线' : '0';
        document.getElementById('alertCount').textContent = stats.active_alerts || 0;
        document.getElementById('avgDeviation').textContent =
            (stats.average_deviation_last_hour || 0).toFixed(2) + '°';
    } catch (e) {
        console.error('加载统计数据失败:', e);
    }
}

function startDataUpdates() {
    setInterval(() => {
        magneticPanel.loadDevices();
        loadStatistics();
    }, CONFIG.UPDATE_INTERVALS.DEVICES);

    setInterval(() => {
        magneticPanel.loadActiveAlerts();
    }, CONFIG.UPDATE_INTERVALS.ALERTS);
}

function startSensorStream() {
    dataService.startSensorStream(
        (data) => {
            if (Array.isArray(data) && data.length > 0) {
                data.forEach(sensorData => {
                    magneticPanel.addSensorDataPoint(sensorData);
                });
            }
        },
        (error) => {
            console.warn('SSE流错误:', error);
        }
    );
}

function showToast(message, type = 'info') {
    const container = document.getElementById('toastContainer');
    if (!container) return;

    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    toast.textContent = message;

    container.appendChild(toast);

    setTimeout(() => {
        toast.classList.add('show');
    }, 10);

    setTimeout(() => {
        toast.classList.remove('show');
        setTimeout(() => {
            container.removeChild(toast);
        }, 300);
    }, 3000);
}

window.addEventListener('beforeunload', () => {
    dataService.closeSensorStream();
    if (window.sinan3d) window.sinan3d.destroy();
    if (magneticPanel) magneticPanel.destroy();
});
