class Sinan3D {
    constructor(canvasId) {
        this.canvas = document.getElementById(canvasId);
        this.scene = null;
        this.camera = null;
        this.renderer = null;
        this.controls = null;
        this.spoonGroup = null;
        this.baseMesh = null;
        this.magnetMesh = null;
        this.magneticFieldLines = [];
        this.northIndicator = null;
        this.southIndicator = null;

        this.currentAzimuth = 0;
        this.targetAzimuth = 0;
        this.expectedAzimuth = 0;

        this.deviation = 0;
        this.fieldIntensity = 55000;

        this.animationId = null;

        this.init();
    }

    init() {
        const rect = this.canvas.parentElement.getBoundingClientRect();
        const width = rect.width;
        const height = rect.height;

        this.scene = new THREE.Scene();
        this.scene.background = new THREE.Color(0x0a0a1a);
        this.scene.fog = new THREE.Fog(0x0a0a1a, 10, 50);

        this.camera = new THREE.PerspectiveCamera(45, width / height, 0.1, 1000);
        this.camera.position.set(8, 6, 8);
        this.camera.lookAt(0, 0, 0);

        this.renderer = new THREE.WebGLRenderer({
            canvas: this.canvas,
            antialias: true,
            alpha: true
        });
        this.renderer.setSize(width, height);
        this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
        this.renderer.shadowMap.enabled = true;
        this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;

        this.controls = new THREE.OrbitControls(this.camera, this.renderer.domElement);
        this.controls.enableDamping = true;
        this.controls.dampingFactor = 0.05;
        this.controls.minDistance = 5;
        this.controls.maxDistance = 20;
        this.controls.maxPolarAngle = Math.PI / 2.1;

        this.setupLighting();
        this.createSinanModel();
        this.createGround();
        this.createMagneticFieldLines();
        this.createDirectionIndicators();

        window.addEventListener('resize', () => this.onResize());

        this.animate();
    }

    setupLighting() {
        const ambientLight = new THREE.AmbientLight(0x404050, 0.5);
        this.scene.add(ambientLight);

        const mainLight = new THREE.DirectionalLight(0xffffff, 1.2);
        mainLight.position.set(10, 15, 10);
        mainLight.castShadow = true;
        mainLight.shadow.mapSize.width = 2048;
        mainLight.shadow.mapSize.height = 2048;
        mainLight.shadow.camera.near = 0.5;
        mainLight.shadow.camera.far = 50;
        mainLight.shadow.camera.left = -10;
        mainLight.shadow.camera.right = 10;
        mainLight.shadow.camera.top = 10;
        mainLight.shadow.camera.bottom = -10;
        this.scene.add(mainLight);

        const fillLight = new THREE.DirectionalLight(0x6688ff, 0.4);
        fillLight.position.set(-10, 5, -10);
        this.scene.add(fillLight);

        const pointLight = new THREE.PointLight(0xff6600, 0.5, 15);
        pointLight.position.set(0, 5, 0);
        this.scene.add(pointLight);
    }

    createSinanModel() {
        this.spoonGroup = new THREE.Group();

        const baseGeometry = new THREE.CylinderGeometry(5, 5.5, 0.3, 64);
        const baseMaterial = new THREE.MeshStandardMaterial({
            color: 0x2a1810,
            roughness: 0.8,
            metalness: 0.1
        });
        this.baseMesh = new THREE.Mesh(baseGeometry, baseMaterial);
        this.baseMesh.position.y = -0.15;
        this.baseMesh.receiveShadow = true;
        this.scene.add(this.baseMesh);

        const topGeometry = new THREE.CylinderGeometry(4.8, 5, 0.05, 64);
        const topMaterial = new THREE.MeshStandardMaterial({
            color: 0x1a0f08,
            roughness: 0.3,
            metalness: 0.8
        });
        const topMesh = new THREE.Mesh(topGeometry, topMaterial);
        topMesh.position.y = 0.025;
        topMesh.receiveShadow = true;
        this.scene.add(topMesh);

        const spoonHandleGeometry = new THREE.CylinderGeometry(0.25, 0.35, 6, 16);
        const spoonBowlGeometry = new THREE.SphereGeometry(1.2, 32, 16, 0, Math.PI * 2, 0, Math.PI / 2);

        const magnetMaterial = new THREE.MeshStandardMaterial({
            color: 0x3a3a4a,
            roughness: 0.4,
            metalness: 0.7
        });

        const handle = new THREE.Mesh(spoonHandleGeometry, magnetMaterial);
        handle.rotation.z = Math.PI / 2;
        handle.position.x = 2;
        handle.castShadow = true;

        const bowl = new THREE.Mesh(spoonBowlGeometry, magnetMaterial);
        bowl.rotation.x = -Math.PI / 2;
        bowl.position.z = -1;
        bowl.castShadow = true;

        const connectorGeometry = new THREE.CylinderGeometry(0.35, 0.35, 0.5, 16);
        const connector = new THREE.Mesh(connectorGeometry, magnetMaterial);
        connector.rotation.z = Math.PI / 2;
        connector.position.x = -0.25;
        connector.castShadow = true;

        const northCapGeometry = new THREE.CylinderGeometry(0.25, 0.3, 0.15, 16);
        const northCapMaterial = new THREE.MeshStandardMaterial({
            color: 0xff3333,
            roughness: 0.3,
            metalness: 0.8,
            emissive: 0xff0000,
            emissiveIntensity: 0.3
        });
        const northCap = new THREE.Mesh(northCapGeometry, northCapMaterial);
        northCap.rotation.z = Math.PI / 2;
        northCap.position.x = 5.05;
        northCap.castShadow = true;

        const southCapGeometry = new THREE.CylinderGeometry(1.0, 0.9, 0.1, 16);
        const southCapMaterial = new THREE.MeshStandardMaterial({
            color: 0x3333ff,
            roughness: 0.3,
            metalness: 0.8,
            emissive: 0x0000ff,
            emissiveIntensity: 0.2
        });
        const southCap = new THREE.Mesh(southCapGeometry, southCapMaterial);
        southCap.rotation.x = -Math.PI / 2;
        southCap.position.z = -1;
        southCap.position.y = 0.05;
        southCap.castShadow = true;

        this.magnetMesh = new THREE.Group();
        this.magnetMesh.add(handle);
        this.magnetMesh.add(bowl);
        this.magnetMesh.add(connector);
        this.magnetMesh.add(northCap);
        this.magnetMesh.add(southCap);

        this.magnetMesh.position.y = 0.5;

        this.spoonGroup.add(this.magnetMesh);
        this.scene.add(this.spoonGroup);
    }

    createGround() {
        const gridHelper = new THREE.GridHelper(20, 40, 0x2a4a6a, 0x1a2a3a);
        gridHelper.position.y = -0.3;
        this.scene.add(gridHelper);

        const groundGeometry = new THREE.PlaneGeometry(30, 30);
        const groundMaterial = new THREE.MeshStandardMaterial({
            color: 0x0a1520,
            roughness: 0.9,
            metalness: 0.1
        });
        const ground = new THREE.Mesh(groundGeometry, groundMaterial);
        ground.rotation.x = -Math.PI / 2;
        ground.position.y = -0.31;
        ground.receiveShadow = true;
        this.scene.add(ground);
    }

    createMagneticFieldLines() {
        const lineCount = 12;
        const pointsPerLine = 50;

        for (let i = 0; i < lineCount; i++) {
            const angle = (i / lineCount) * Math.PI * 2;
            const points = [];

            for (let j = 0; j <= pointsPerLine; j++) {
                const t = j / pointsPerLine;
                const radius = 3 + t * 5;
                const height = Math.sin(t * Math.PI) * 2;

                const x = Math.cos(angle) * radius;
                const z = Math.sin(angle) * radius;
                const y = height;

                points.push(new THREE.Vector3(x, y, z));
            }

            const geometry = new THREE.BufferGeometry().setFromPoints(points);
            const material = new THREE.LineBasicMaterial({
                color: 0x4488ff,
                transparent: true,
                opacity: 0.4
            });

            const line = new THREE.Line(geometry, material);
            this.magneticFieldLines.push(line);
            this.scene.add(line);
        }
    }

    createDirectionIndicators() {
        const arrowLength = 2;
        const arrowHelper = new THREE.ArrowHelper(
            new THREE.Vector3(1, 0, 0),
            new THREE.Vector3(0, 0.1, 0),
            arrowLength,
            0xff3333,
            0.3,
            0.15
        );
        this.scene.add(arrowHelper);

        const northLabelGeometry = new THREE.PlaneGeometry(0.5, 0.3);
        const canvas = document.createElement('canvas');
        canvas.width = 128;
        canvas.height = 64;
        const ctx = canvas.getContext('2d');
        ctx.fillStyle = '#ff3333';
        ctx.font = 'bold 48px Arial';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText('N', 64, 32);

        const texture = new THREE.CanvasTexture(canvas);
        const northLabelMaterial = new THREE.MeshBasicMaterial({
            map: texture,
            transparent: true,
            side: THREE.DoubleSide
        });

        this.northIndicator = new THREE.Mesh(northLabelGeometry, northLabelMaterial);
        this.northIndicator.position.set(arrowLength + 0.5, 0.3, 0);
        this.scene.add(this.northIndicator);
    }

    updateDeviation(deviation, isAlert) {
        this.deviation = deviation;

        const deviationElement = document.getElementById('deviationValue');
        const statusElement = document.getElementById('deviationStatus');
        const labelElement = document.getElementById('deviationLabel');

        if (deviationElement) {
            deviationElement.textContent = deviation.toFixed(2) + '°';
            deviationElement.className = 'deviation-value';

            if (deviation >= CONFIG.THRESHOLDS.CRITICAL) {
                deviationElement.classList.add('critical');
                statusElement.textContent = '严重超限';
                statusElement.className = 'deviation-status critical';
                labelElement.style.borderColor = 'rgba(255, 82, 82, 0.6)';
            } else if (deviation >= CONFIG.THRESHOLDS.WARNING) {
                deviationElement.classList.add('warning');
                statusElement.textContent = '超限告警';
                statusElement.className = 'deviation-status warning';
                labelElement.style.borderColor = 'rgba(255, 152, 0, 0.6)';
            } else {
                statusElement.textContent = '正常';
                statusElement.className = 'deviation-status';
                labelElement.style.borderColor = 'rgba(100, 150, 255, 0.3)';
            }
        }
    }

    setTargetAzimuth(azimuth, expectedAzimuth) {
        this.targetAzimuth = azimuth;
        this.expectedAzimuth = expectedAzimuth || 0;

        const currentElement = document.getElementById('currentAzimuth');
        const expectedElement = document.getElementById('expectedAzimuth');

        if (currentElement) {
            currentElement.textContent = azimuth.toFixed(2) + '°';
        }
        if (expectedElement && expectedAzimuth !== undefined) {
            expectedElement.textContent = expectedAzimuth.toFixed(2) + '°';
        }
    }

    updateSensorData(data) {
        this.currentAzimuth = this.calculateAzimuthFromMoment(
            data.magnetic_moment_x,
            data.magnetic_moment_y
        );

        this.setTargetAzimuth(this.currentAzimuth, 0);
        this.updateDeviation(data.pointing_deviation, data.is_alert);

        const momentElement = document.getElementById('momentMagnitude');
        const remanenceElement = document.getElementById('currentRemanence');
        const tempElement = document.getElementById('currentTemp');
        const fieldElement = document.getElementById('fieldIntensity');

        if (momentElement) {
            momentElement.textContent = data.magnetic_moment_magnitude.toFixed(4) + ' A·m²';
        }
        if (remanenceElement) {
            remanenceElement.textContent = data.remanence.toFixed(4) + ' T';
        }
        if (tempElement) {
            tempElement.textContent = data.environment_temp.toFixed(1) + '°C';
        }
        if (fieldElement) {
            fieldElement.textContent = Math.round(this.fieldIntensity) + ' nT';
        }

        this.updateMagneticFieldVisualization(data);
    }

    calculateAzimuthFromMoment(mx, my) {
        const azimuth = Math.atan2(my, mx) * 180 / Math.PI;
        return (azimuth + 360) % 360;
    }

    updateMagneticFieldVisualization(data) {
        const intensityFactor = Math.min(data.remanence / 0.5, 1.5);

        this.magneticFieldLines.forEach((line, index) => {
            const opacity = 0.2 + intensityFactor * 0.3;
            line.material.opacity = opacity;

            const hue = (0.6 + data.pointing_deviation / 20 * 0.4) % 1;
            const color = new THREE.Color().setHSL(hue, 0.8, 0.5);
            line.material.color = color;
        });
    }

    setFieldIntensity(intensity) {
        this.fieldIntensity = intensity;
        const fieldElement = document.getElementById('fieldIntensity');
        if (fieldElement) {
            fieldElement.textContent = Math.round(intensity) + ' nT';
        }
    }

    onResize() {
        const rect = this.canvas.parentElement.getBoundingClientRect();
        const width = rect.width;
        const height = rect.height;

        this.camera.aspect = width / height;
        this.camera.updateProjectionMatrix();
        this.renderer.setSize(width, height);
    }

    animate() {
        this.animationId = requestAnimationFrame(() => this.animate());

        if (this.spoonGroup) {
            const currentRotation = this.spoonGroup.rotation.y;
            const targetRotation = -(this.targetAzimuth * Math.PI / 180) + Math.PI / 2;

            const diff = targetRotation - currentRotation;
            const normalizedDiff = Math.atan2(Math.sin(diff), Math.cos(diff));

            this.spoonGroup.rotation.y += normalizedDiff * 0.05;

            const bobOffset = Math.sin(Date.now() * 0.001) * 0.02;
            this.magnetMesh.position.y = 0.5 + bobOffset;

            const deviationFactor = this.deviation / 10;
            this.spoonGroup.rotation.x = Math.sin(Date.now() * 0.0015) * 0.02 * deviationFactor;
            this.spoonGroup.rotation.z = Math.cos(Date.now() * 0.001) * 0.01 * deviationFactor;
        }

        if (this.northIndicator) {
            this.northIndicator.lookAt(this.camera.position);
        }

        this.controls.update();
        this.renderer.render(this.scene, this.camera);
    }

    destroy() {
        if (this.animationId) {
            cancelAnimationFrame(this.animationId);
        }

        this.scene.traverse((object) => {
            if (object.geometry) object.geometry.dispose();
            if (object.material) {
                if (Array.isArray(object.material)) {
                    object.material.forEach(m => m.dispose());
                } else {
                    object.material.dispose();
                }
            }
        });

        this.renderer.dispose();
    }
}
