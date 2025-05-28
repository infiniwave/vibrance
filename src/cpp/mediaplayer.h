#ifndef MEDIAPLAYER_H
#define MEDIAPLAYER_H

#include <QWidget>
#include <QHBoxLayout>
#include <QFrame>
#include <QLabel>
#include <QVBoxLayout>
#include <QPushButton>
#include <QSlider>
#include <QFont>
#include <QMetaObject>
#include <QObject>
#include <string>
#include <QToolButton>
#include "volumeflyout.h" 
#include <QMainWindow>
#include <QSvgRenderer>
#include <QPainter>

class MediaPlayer : public QWidget
{
    Q_OBJECT

public:
    MediaPlayer(QWidget *parent = nullptr);
    ~MediaPlayer();

private:
    // UI elements
    QHBoxLayout *horizontalLayout;
    QFrame *frame;
    QLabel *trackTitle;
    QLabel *trackArtists;
    QVBoxLayout *trackDetails;
    QVBoxLayout *verticalLayout_2;
    QHBoxLayout *horizontalLayout_2;
    QPushButton *pauseButton;
    QPushButton *previousButton;
    QPushButton *nextButton;
    QSlider *trackProgress;
    QHBoxLayout *trackProgressContainer;
    QLabel *elapsedDuration;
    QLabel *totalDuration;
    QToolButton *volumeButton;
    VolumeFlyout *volumeFlyout;

    void setupUi();

signals:
    void progressChanged(double value);

public slots:
    void setProgress(double value);
    void setTrack(std::string title, std::string artists, std::string album, double duration);
    void setPaused(bool paused);
};

#endif // MEDIAPLAYER_H
