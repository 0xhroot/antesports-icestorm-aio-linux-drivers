#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QMainWindow>
#include <QLabel>
#include <QTimer>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QGroupBox>
#include <QProgressBar>
#include <QPushButton>
#include <QComboBox>
#include <QSpinBox>
#include <QStatusBar>

class MainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);
    ~MainWindow() override = default;

private slots:
    void refreshDisplay();
    void onBrightnessChanged(int value);
    void onOrientationChanged(int index);

private:
    void setupUI();
    void setSensorValue(QLabel *label, const QString &text, const QString &unit);

    // Device info
    QLabel *m_deviceStatus;
    QLabel *m_firmwareVersion;

    // CPU group
    QLabel *m_cpuTemp;
    QLabel *m_cpuUsage;
    QLabel *m_cpuFreq;
    QLabel *m_cpuPower;
    QProgressBar *m_cpuUsageBar;

    // GPU group
    QLabel *m_gpuTemp;
    QLabel *m_gpuUsage;
    QLabel *m_gpuFreq;
    QLabel *m_gpuPower;
    QProgressBar *m_gpuUsageBar;

    // Fan group
    QLabel *m_fanRpm;
    QLabel *m_pumpRpm;

    // System
    QLabel *m_ramUsage;
    QLabel *m_uptime;

    // Controls
    QSpinBox *m_brightnessSpin;
    QComboBox *m_orientationCombo;
    QPushButton *m_shutdownBtn;

    QTimer *m_timer;
};

#endif // MAINWINDOW_H
