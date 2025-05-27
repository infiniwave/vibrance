#include "mediaplayer.h"
#include "volumeflyout.h"
#include "../../target/cxxbridge/vibrance/src/main.rs.h"

MediaPlayer::MediaPlayer(QWidget *parent)
    : QWidget(parent)
{
    setupUi();
}

MediaPlayer::~MediaPlayer()
{
    // Qt will delete child widgets automatically
}

int lastSliderValue = 0;
bool isSliderBeingDragged = false;
double trackLength = 0.0;

// format seconds as mm:ss
std::string formatDuration(double seconds) {
    int totalSeconds = static_cast<int>(seconds);
    int minutes = totalSeconds / 60;
    int secs = totalSeconds % 60;
    char buffer[16];
    snprintf(buffer, sizeof(buffer), "%02d:%02d", minutes, secs);
    return std::string(buffer);
}
void MediaPlayer::setupUi()
{
    if (objectName().isEmpty())
        setObjectName("MediaPlayer");
    resize(1021, 150);
    setMinimumSize(QSize(0, 150));
    setMaximumSize(QSize(16777215, 150));
    horizontalLayout = new QHBoxLayout(this);
    horizontalLayout->setObjectName("horizontalLayout");
    frame = new QFrame(this);
    frame->setObjectName("frame");
    frame->setEnabled(true);
    frame->setMinimumSize(QSize(120, 120));
    frame->setMaximumSize(QSize(120, 120));
    frame->setFrameShape(QFrame::Shape::StyledPanel);
    frame->setFrameShadow(QFrame::Shadow::Raised);
    horizontalLayout->addWidget(frame);
    trackTitle = new QLabel(this);
    trackTitle->setObjectName("trackTitle");
    trackTitle->setMaximumSize(QSize(200, 16777215));
    QFont font;
    font.setFamilies({QString::fromUtf8("HONOR Sans CN")});
    font.setPointSize(16);
    trackTitle->setFont(font);
    trackTitle->setText("Track Title");
    horizontalLayout->addWidget(trackTitle);
    verticalLayout_2 = new QVBoxLayout();
    verticalLayout_2->setObjectName("verticalLayout_2");
    horizontalLayout_2 = new QHBoxLayout();
    horizontalLayout_2->setObjectName("horizontalLayout_2");
    pushButton_2 = new QPushButton(this);
    pushButton_2->setObjectName("pushButton_2");
    QFont font1;
    font1.setFamilies({QString::fromUtf8("HONOR Sans")});
    pushButton_2->setFont(font1);
    pushButton_2->setText("Play/Pause");
    connect(pushButton_2, &QPushButton::clicked, this, [this]() {
        pause();
    });
    horizontalLayout_2->addWidget(pushButton_2);
    pushButton = new QPushButton(this);
    pushButton->setObjectName("pushButton");
    pushButton->setText("Stop");
    horizontalLayout_2->addWidget(pushButton);
    volumeButton = new QToolButton(this);
    volumeButton->setObjectName("volumeButton");
    volumeButton->setText("Volume");
    horizontalLayout_2->addWidget(volumeButton);
    volumeFlyout = new VolumeFlyout(this);
    volumeFlyout->setWindowFlags(Qt::Popup);
    connect(volumeButton, &QToolButton::clicked, this, [this]() {
        if (volumeFlyout->isVisible()) {
            volumeFlyout->hide();
        } else {
            QPoint globalPos = volumeButton->mapToGlobal(QPoint(0, volumeButton->height()));
            volumeFlyout->move(globalPos.x(), globalPos.y());
            volumeFlyout->show();
            volumeFlyout->raise();
        }
    });
    connect(volumeFlyout, &VolumeFlyout::volumeChanged, this, [this](int value) {
        set_volume(value);
    });
    verticalLayout_2->addLayout(horizontalLayout_2);


    trackProgressContainer = new QHBoxLayout();
    trackProgressContainer->setObjectName("trackProgressContainer");
    elapsedDuration = new QLabel(this);
    elapsedDuration->setObjectName("elapsedDuration");
    elapsedDuration->setText("00:00");
    trackProgressContainer->addWidget(elapsedDuration, 0, Qt::AlignmentFlag::AlignLeft);

    trackProgress = new QSlider(this);
    trackProgress->setObjectName("trackProgress");
    trackProgress->setMaximumSize(QSize(16777215, 16777215));
    trackProgress->setOrientation(Qt::Orientation::Horizontal);
    trackProgress->setMinimum(0);
    trackProgress->setMaximum(100000); 
    connect(trackProgress, &QSlider::sliderPressed, this, [this]() {
        isSliderBeingDragged = true;
        lastSliderValue = this->trackProgress->value();
    });
    connect(trackProgress, &QSlider::sliderReleased, this, [this]() {
        isSliderBeingDragged = false;
        seek(lastSliderValue / 100000.0);
    });
    connect(trackProgress, &QSlider::sliderMoved, this, [this](int value) {
        lastSliderValue = value;
    });
    connect(trackProgress, &QSlider::valueChanged, this, [this](int value) {
        this->elapsedDuration->setText(QString::fromStdString(formatDuration(value / 100000.0 * trackLength)));
    });
    trackProgressContainer->addWidget(trackProgress);
    totalDuration = new QLabel(this);
    totalDuration->setObjectName("totalDuration");
    totalDuration->setText("00:00");
    trackProgressContainer->addWidget(totalDuration, 0, Qt::AlignmentFlag::AlignRight);
    verticalLayout_2->addLayout(trackProgressContainer);
    horizontalLayout->addLayout(verticalLayout_2);
    horizontalLayout->setStretch(0, 2);
    horizontalLayout->setStretch(1, 5);
    horizontalLayout->setStretch(2, 5);
}

void MediaPlayer::setProgress(double value) {
    emit progressChanged(value);
    if (trackProgress && !isSliderBeingDragged) {
        trackProgress->setValue(int(value* 100000));
    }
}

void MediaPlayer::setTrack(std::string title, std::string artists, std::string album, double duration) {
    trackTitle->setText(QString::fromStdString(title));
    totalDuration->setText(QString::fromStdString(formatDuration(duration)));
    trackLength = duration;
}

void MediaPlayer::initializeVolume(int initialVolume) {
    if (volumeFlyout) {
        volumeFlyout->initializeVolume(initialVolume);
    }
}

void MediaPlayer::setPaused(bool paused) {
    if (paused) {
        pushButton_2->setText("Play");
    } else {
        pushButton_2->setText("Pause");
    }
}
