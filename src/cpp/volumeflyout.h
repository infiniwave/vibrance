#ifndef VOLUMEFLYOUT_H
#define VOLUMEFLYOUT_H

#include <QApplication>
#include <QWidget>
#include <QToolButton>
#include <QSlider>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QPoint>
#include <QDebug>

class VolumeFlyout : public QWidget
{
    Q_OBJECT

public:
    VolumeFlyout(QWidget *parent = nullptr);
    ~VolumeFlyout();
    void initializeVolume(int initialVolume);

private:
    // UI elements
    QSlider *slider;

    void setupUi();

signals:
    void volumeChanged(int value);
};

#endif // MEDIAPLAYER_H
