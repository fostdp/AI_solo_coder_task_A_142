const CONFIG = {
    API_BASE_URL: 'http://localhost:8080',
    ENDPOINTS: {
        HEALTH: '/health',
        SENSOR_DATA: '/api/v1/sensor',
        SENSOR_DATA_QUERY: '/api/v1/sensor/data',
        SENSOR_LATEST: '/api/v1/sensor/latest',
        SENSOR_STREAM: '/api/v1/sensor/stream',
        DEVICES: '/api/v1/devices',
        DEVICE_STATUS: '/api/v1/device/status',
        GEOMAGNETIC_FIELD: '/api/v1/geomagnetic/field',
        VECTOR_FIELD: '/api/v1/geomagnetic/vectorfield',
        SECULAR_VARIATION: '/api/v1/geomagnetic/secular',
        SIMULATION_POINTING: '/api/v1/simulation/pointing',
        SIMULATION_RESULTS: '/api/v1/simulation/results',
        ALERTS_ACTIVE: '/api/v1/alerts/active',
        ALERTS_ACKNOWLEDGE: '/api/v1/alerts/acknowledge',
        STATISTICS: '/api/v1/statistics'
    },
    THRESHOLDS: {
        WARNING: 5.0,
        CRITICAL: 10.0
    },
    UPDATE_INTERVALS: {
        DEVICES: 5000,
        ALERTS: 3000,
        STATISTICS: 10000
    }
};
