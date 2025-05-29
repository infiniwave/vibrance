#include "mediaplayer.h"
#include "volumeflyout.h"
#include "../../target/cxxbridge/vibrance/src/main.rs.h"
#include "mainwindow.h"
#include <QSvgRenderer>

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

QIcon getIcon(const char* iconPath) {
    QIcon icon;
    if (QPalette().color(QPalette::Window).lightness() < 128) {
        // svg must be rendered in white for dark mode
        QSvgRenderer renderer(QString::fromUtf8(iconPath));
        QSize size(48, 48);
        QPixmap pixmap(size);
        pixmap.fill(Qt::transparent);
        QPainter painter(&pixmap);
        renderer.render(&painter);
        painter.setCompositionMode(QPainter::CompositionMode_SourceIn);
        painter.fillRect(pixmap.rect(), Qt::white);
        painter.end();
        icon.addPixmap(pixmap, QIcon::Normal, QIcon::Off);
    } else {
        icon.addFile(QString::fromUtf8(iconPath), QSize(48, 48), QIcon::Normal, QIcon::Off);
    }
    return icon;
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
    trackTitle->setMaximumSize(QSize(300, 16777215));
    QFont font;
    font.setPointSize(16);
    trackTitle->setFont(font);
    trackTitle->setText("Track Title");
    trackArtists = new QLabel(this);
    trackArtists->setObjectName("trackArtists");
    trackArtists->setMaximumSize(QSize(200, 16777215));
    trackArtists->setText("Track Artists");
    QPalette palette = trackArtists->palette();
    palette.setColor(QPalette::WindowText, QColor(150, 150, 150));
    trackArtists->setPalette(palette);
    trackDetails = new QVBoxLayout();
    trackDetails->setObjectName("trackDetails");
    trackDetails->addWidget(trackTitle);
    trackDetails->addWidget(trackArtists);
    horizontalLayout->addLayout(trackDetails);
    verticalLayout_2 = new QVBoxLayout();
    verticalLayout_2->setObjectName("verticalLayout_2");
    horizontalLayout_2 = new QHBoxLayout();
    horizontalLayout_2->setObjectName("horizontalLayout_2");
    previousButton = new QPushButton(this);
    previousButton->setObjectName("previousButton");
    previousButton->setIcon(getIcon(":/previous.svg"));
    previousButton->setToolTip("Previous");
    previousButton->setSizePolicy(QSizePolicy::Fixed, QSizePolicy::Fixed);
    previousButton->setStyleSheet("QPushButton { background: transparent; padding: 8px; border-radius: 4px; }"
                                "QPushButton:hover { background: rgba(255, 255, 255, 0.1); }"
                                "QPushButton:pressed { background: rgba(255, 255, 255, 0.2); }");
    horizontalLayout_2->addWidget(previousButton);
    pauseButton = new QPushButton(this);
    pauseButton->setObjectName("pauseButton");
    pauseButton->setIcon(getIcon(":/play.svg"));
    pauseButton->setToolTip("Play/Pause");
    // pauseButton->setSizePolicy(QSizePolicy::Fixed, QSizePolicy::Fixed);
    pauseButton->setStyleSheet("QPushButton { background: transparent; padding: 8px; border-radius: 4px; }"
                                "QPushButton:hover { background: rgba(255, 255, 255, 0.1); }"
                                "QPushButton:pressed { background: rgba(255, 255, 255, 0.2); }");
    connect(pauseButton, &QPushButton::clicked, this, [this]() {
        pause();
    });
    horizontalLayout_2->addWidget(pauseButton);
    nextButton = new QPushButton(this);
    nextButton->setObjectName("nextButton");
    nextButton->setIcon(getIcon(":/next.svg"));
    nextButton->setToolTip("Next");
    nextButton->setSizePolicy(QSizePolicy::Fixed, QSizePolicy::Fixed);
    nextButton->setStyleSheet("QPushButton { background: transparent; padding: 8px; border-radius: 4px; }"
                                "QPushButton:hover { background: rgba(255, 255, 255, 0.1); }"
                                "QPushButton:pressed { background: rgba(255, 255, 255, 0.2); }");
    horizontalLayout_2->addWidget(nextButton);
    volumeButton = new QToolButton(this);
    volumeButton->setObjectName("volumeButton");
    volumeButton->setStyleSheet("QToolButton { background: transparent; padding: 8px; border-radius: 4px; }"
                                "QToolButton:hover { background: rgba(255, 255, 255, 0.1); }"
                                "QToolButton:pressed { background: rgba(255, 255, 255, 0.2); }");
    QIcon volumeIcon = getIcon(":/speaker_2.svg");
    volumeButton->setIcon(volumeIcon);
    volumeButton->setToolTip("Volume");
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
    QMainWindow* mainWin = qobject_cast<QMainWindow*>(this->window());
    if (mainWin) {
        auto mw = qobject_cast<MainWindow*>(mainWin);
        if (mw) {
            mw->updateLyricHighlight(value* trackLength* 1000);
        }
    }
}

void MediaPlayer::setTrack(std::string title, std::string artists, std::string album, double duration) {
    trackTitle->setText(QString::fromStdString(title));
    trackArtists->setText(QString::fromStdString(artists));
    totalDuration->setText(QString::fromStdString(formatDuration(duration)));
    trackLength = duration;    
    QMainWindow* mainWin = qobject_cast<QMainWindow*>(this->window());
    if (mainWin) {
        auto mw = qobject_cast<MainWindow*>(mainWin);
        if (mw) {
            mw->loadLyrics();
        }
    }
}


void MediaPlayer::setPaused(bool paused) {
    if (paused) {
        pauseButton->setIcon(getIcon(":/play.svg"));
    } else {
        pauseButton->setIcon(getIcon(":/pause.svg"));
    }
}
