#include "mainwindow.h"
#include <QApplication>
#include <QStyle>
#include <QFile>
#include <QProcess>
#include <QDateTime>
#include <QDebug>
#include <cmath>

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
{
    setWindowTitle("ANTESPORTS Monitor");
    setMinimumSize(480, 640);
    resize(520, 700);
    setupUI();

    m_timer = new QTimer(this);
    connect(m_timer, &QTimer::timeout, this, &MainWindow::refreshDisplay);
    m_timer->start(1000);

    statusBar()->showMessage("Ready");
}

void MainWindow::setupUI() {
    auto *centralWidget = new QWidget(this);
    auto *mainLayout = new QVBoxLayout(centralWidget);
    mainLayout->setSpacing(12);
    mainLayout->setContentsMargins(16, 16, 16, 16);

    // Title
    auto *titleLabel = new QLabel("<h1>ANTESPORTS Monitor</h1>", this);
    titleLabel->setAlignment(Qt::AlignCenter);
    mainLayout->addWidget(titleLabel);

    // Device info
    m_deviceStatus = new QLabel("Device: Not connected", this);
    m_deviceStatus->setStyleSheet("color: #888; font-size: 12px;");
    m_deviceStatus->setAlignment(Qt::AlignCenter);
    mainLayout->addWidget(m_deviceStatus);

    m_firmwareVersion = new QLabel("", this);
    m_firmwareVersion->setAlignment(Qt::AlignCenter);
    mainLayout->addWidget(m_firmwareVersion);

    // CPU Group
    auto *cpuGroup = new QGroupBox("CPU", this);
    auto *cpuLayout = new QVBoxLayout(cpuGroup);

    auto *cpuTempLayout = new QHBoxLayout();
    m_cpuTemp = new QLabel("--", this);
    m_cpuTemp->setStyleSheet("font-size: 36px; font-weight: bold;");
    cpuTempLayout->addWidget(m_cpuTemp);
    cpuTempLayout->addStretch();
    cpuLayout->addLayout(cpuTempLayout);

    m_cpuUsageBar = new QProgressBar(this);
    m_cpuUsageBar->setRange(0, 100);
    m_cpuUsageBar->setTextVisible(true);
    cpuLayout->addWidget(m_cpuUsageBar);

    auto *cpuInfoLayout = new QHBoxLayout();
    m_cpuUsage = new QLabel("Usage: --%", this);
    m_cpuFreq = new QLabel("Freq: -- MHz", this);
    m_cpuPower = new QLabel("Power: -- W", this);
    cpuInfoLayout->addWidget(m_cpuUsage);
    cpuInfoLayout->addWidget(m_cpuFreq);
    cpuInfoLayout->addWidget(m_cpuPower);
    cpuLayout->addLayout(cpuInfoLayout);
    mainLayout->addWidget(cpuGroup);

    // GPU Group
    auto *gpuGroup = new QGroupBox("GPU", this);
    auto *gpuLayout = new QVBoxLayout(gpuGroup);

    auto *gpuTempLayout = new QHBoxLayout();
    m_gpuTemp = new QLabel("--", this);
    m_gpuTemp->setStyleSheet("font-size: 36px; font-weight: bold;");
    gpuTempLayout->addWidget(m_gpuTemp);
    gpuTempLayout->addStretch();
    gpuLayout->addLayout(gpuTempLayout);

    m_gpuUsageBar = new QProgressBar(this);
    m_gpuUsageBar->setRange(0, 100);
    m_gpuUsageBar->setTextVisible(true);
    gpuLayout->addWidget(m_gpuUsageBar);

    auto *gpuInfoLayout = new QHBoxLayout();
    m_gpuUsage = new QLabel("Usage: --%", this);
    m_gpuFreq = new QLabel("Freq: -- MHz", this);
    m_gpuPower = new QLabel("Power: -- W", this);
    gpuInfoLayout->addWidget(m_gpuUsage);
    gpuInfoLayout->addWidget(m_gpuFreq);
    gpuInfoLayout->addWidget(m_gpuPower);
    gpuLayout->addLayout(gpuInfoLayout);
    mainLayout->addWidget(gpuGroup);

    // Fan Group
    auto *fanGroup = new QGroupBox("Cooling", this);
    auto *fanLayout = new QHBoxLayout(fanGroup);
    m_fanRpm = new QLabel("Fan: -- RPM", this);
    m_fanRpm->setStyleSheet("font-size: 18px;");
    m_pumpRpm = new QLabel("Pump: -- RPM", this);
    m_pumpRpm->setStyleSheet("font-size: 18px;");
    fanLayout->addWidget(m_fanRpm);
    fanLayout->addWidget(m_pumpRpm);
    mainLayout->addWidget(fanGroup);

    // System Group
    auto *sysGroup = new QGroupBox("System", this);
    auto *sysLayout = new QHBoxLayout(sysGroup);
    m_ramUsage = new QLabel("RAM: --%", this);
    m_uptime = new QLabel("Uptime: --", this);
    sysLayout->addWidget(m_ramUsage);
    sysLayout->addWidget(m_uptime);
    mainLayout->addWidget(sysGroup);

    // Controls
    auto *controlsGroup = new QGroupBox("Display Control", this);
    auto *controlsLayout = new QVBoxLayout(controlsGroup);

    auto *brightnessLayout = new QHBoxLayout();
    brightnessLayout->addWidget(new QLabel("Brightness:", this));
    m_brightnessSpin = new QSpinBox(this);
    m_brightnessSpin->setRange(0, 100);
    m_brightnessSpin->setValue(50);
    brightnessLayout->addWidget(m_brightnessSpin);
    brightnessLayout->addStretch();
    controlsLayout->addLayout(brightnessLayout);

    auto *orientationLayout = new QHBoxLayout();
    orientationLayout->addWidget(new QLabel("Orientation:", this));
    m_orientationCombo = new QComboBox(this);
    m_orientationCombo->addItem("Normal");
    m_orientationCombo->addItem("Rotate 90°");
    m_orientationCombo->addItem("Rotate 180°");
    m_orientationCombo->addItem("Rotate 270°");
    orientationLayout->addWidget(m_orientationCombo);
    orientationLayout->addStretch();
    controlsLayout->addLayout(orientationLayout);

    mainLayout->addWidget(controlsGroup);

    // Shutdown button
    m_shutdownBtn = new QPushButton("Clear Display", this);
    mainLayout->addWidget(m_shutdownBtn);

    mainLayout->addStretch();
    setCentralWidget(centralWidget);

    // Connections
    connect(m_brightnessSpin, QOverload<int>::of(&QSpinBox::valueChanged),
            this, &MainWindow::onBrightnessChanged);
    connect(m_orientationCombo, QOverload<int>::of(&QComboBox::currentIndexChanged),
            this, &MainWindow::onOrientationChanged);
}

void MainWindow::setSensorValue(QLabel *label, const QString &text, const QString &unit) {
    label->setText(text + " " + unit);
}

void MainWindow::refreshDisplay() {
    static int counter = 0;
    counter++;

    // Read sensors via sysfs
    QProcess cpuTempProc;
    cpuTempProc.start("cat", QStringList() << "/sys/class/thermal/thermal_zone0/temp");
    cpuTempProc.waitForFinished(500);
    QString cpuTempStr = QString::fromUtf8(cpuTempProc.readAllStandardOutput()).trimmed();
    bool ok;
    double cpuTemp = cpuTempStr.toDouble(&ok) / 1000.0;
    if (ok) {
        m_cpuTemp->setText(QString("%1°C").arg(cpuTemp, 0, 'f', 1));
    }

    // CPU usage from /proc/stat
    static unsigned long prevTotal = 0, prevIdle = 0;
    QProcess statProc;
    statProc.start("cat", QStringList() << "/proc/stat");
    statProc.waitForFinished(500);
    QString stat = QString::fromUtf8(statProc.readAllStandardOutput());
    QStringList lines = stat.split('\n', Qt::SkipEmptyParts);
    if (!lines.isEmpty()) {
        QStringList fields = lines[0].split(' ', Qt::SkipEmptyParts);
        if (fields.size() >= 5) {
            unsigned long total = 0;
            for (int i = 1; i < fields.size(); i++) total += fields[i].toULong();
            unsigned long idle = fields[4].toULong();
            if (prevTotal > 0) {
                double usage = (double)(total - prevTotal - (idle - prevIdle)) / (double)(total - prevTotal) * 100.0;
                m_cpuUsageBar->setValue((int)usage);
                m_cpuUsage->setText(QString("Usage: %1%").arg(usage, 0, 'f', 1));
            }
            prevTotal = total;
            prevIdle = idle;
        }
    }

    // GPU temp
    QProcess gpuTempProc;
    gpuTempProc.start("cat", QStringList() << "/sys/class/drm/card0/device/hwmon/hwmon0/temp1_input");
    gpuTempProc.waitForFinished(500);
    QString gpuTempStr = QString::fromUtf8(gpuTempProc.readAllStandardOutput()).trimmed();
    double gpuTemp = gpuTempStr.toDouble(&ok) / 1000.0;
    if (ok) {
        m_gpuTemp->setText(QString("%1°C").arg(gpuTemp, 0, 'f', 1));
    }

    // Fan RPM
    QProcess fanProc;
    fanProc.start("bash", QStringList() << "-c" << "cat /sys/class/hwmon/hwmon*/fan*_input 2>/dev/null | head -1");
    fanProc.waitForFinished(500);
    QString fanStr = QString::fromUtf8(fanProc.readAllStandardOutput()).trimmed();
    if (!fanStr.isEmpty()) {
        m_fanRpm->setText(QString("Fan: %1 RPM").arg(fanStr));
    }

    // RAM
    QProcess memProc;
    memProc.start("bash", QStringList() << "-c" << "free | grep Mem | awk '{print $3/$2 * 100.0}'");
    memProc.waitForFinished(500);
    QString memStr = QString::fromUtf8(memProc.readAllStandardOutput()).trimmed();
    if (!memStr.isEmpty()) {
        double memPct = memStr.toDouble();
        m_ramUsage->setText(QString("RAM: %1%").arg(memPct, 0, 'f', 1));
    }

    // Uptime
    QProcess uptimeProc;
    uptimeProc.start("cat", QStringList() << "/proc/uptime");
    uptimeProc.waitForFinished(500);
    QString uptimeStr = QString::fromUtf8(uptimeProc.readAllStandardOutput()).trimmed();
    double uptimeSecs = uptimeStr.split(' ').first().toDouble();
    int hours = (int)uptimeSecs / 3600;
    int mins = ((int)uptimeSecs % 3600) / 60;
    m_uptime->setText(QString("Uptime: %1h %2m").arg(hours).arg(mins));

    // Device status
    QProcess lsusbProc;
    lsusbProc.start("bash", QStringList() << "-c" << "lsusb -d 5131:2007 2>/dev/null || lsusb -d 2022:0522 2>/dev/null || echo 'Not found'");
    lsusbProc.waitForFinished(500);
    QString usbStatus = QString::fromUtf8(lsusbProc.readAllStandardOutput()).trimmed();
    if (usbStatus.contains("Not found")) {
        m_deviceStatus->setText("Device: Not connected");
        m_deviceStatus->setStyleSheet("color: #e74c3c; font-size: 12px;");
    } else {
        m_deviceStatus->setText("Device: Connected");
        m_deviceStatus->setStyleSheet("color: #2ecc71; font-size: 12px;");
    }
}

void MainWindow::onBrightnessChanged(int value) {
    statusBar()->showMessage(QString("Brightness set to %%").arg(value), 3000);
}

void MainWindow::onOrientationChanged(int index) {
    statusBar()->showMessage(QString("Orientation changed to mode %1").arg(index), 3000);
}
