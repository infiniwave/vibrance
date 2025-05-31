#include "trackitem.h"
#include "../../target/cxxbridge/vibrance/src/main.rs.h"
#include <QPainterPath>
#include <QGuiApplication>
#include <QScreen>

QPixmap getAlbumArtPixmap(QByteArray base64ImageData, int size) {
    QImage image;
    image.loadFromData(base64ImageData);

    qreal dpr = 1.0;
    if (QGuiApplication::primaryScreen()) {
        dpr = QGuiApplication::primaryScreen()->devicePixelRatio();
    }

    QPixmap pixmap = QPixmap::fromImage(image).scaled(QSize(size * dpr, size * dpr), Qt::KeepAspectRatioByExpanding, Qt::SmoothTransformation);
    QPixmap roundedPixmap(QSize(size * dpr, size * dpr));
    roundedPixmap.fill(Qt::transparent);
    QPainter painter(&roundedPixmap);
    painter.setRenderHint(QPainter::Antialiasing);
    QPainterPath path;
    path.addRoundedRect(roundedPixmap.rect(), 8 * dpr, 8 * dpr);
    painter.setClipPath(path);
    painter.drawPixmap(0, 0, pixmap);
    painter.end();
    roundedPixmap.setDevicePixelRatio(dpr);
    return roundedPixmap;
}

TrackItem::TrackItem(std::string id, const QString &title, const QString &artist, std::string albumArtData, QWidget *parent)
    : QWidget(parent)
{
    layout = new QHBoxLayout(this);

    // Album Art
    int size = 60; 
    albumArtFrame = new QFrame(this);
    albumArtFrame->setEnabled(true);
    albumArtFrame->setMinimumSize(QSize(size, size));
    albumArtFrame->setMaximumSize(QSize(size, size));
    albumArt = new QLabel(albumArtFrame);
    albumArt->setGeometry(QRect(0, 0, size, size));
    albumArt->setMinimumSize(QSize(size, size));
    albumArt->setMaximumSize(QSize(size, size));
    QByteArray base64ImageData = QByteArray::fromBase64(albumArtData.c_str());
    if (!base64ImageData.isEmpty()) {
        albumArt->setPixmap(getAlbumArtPixmap(base64ImageData, size));
    }
    layout->addWidget(albumArtFrame);

    // Title and Artist
    textLayout = new QVBoxLayout;
    titleLabel = new QLabel(title);
    artistLabel = new QLabel(artist);
    titleLabel->setStyleSheet("font-weight: bold; color: white; background-color: transparent;");
    artistLabel->setStyleSheet("color: gray; background-color: transparent;");
    textLayout->addWidget(titleLabel);
    textLayout->addWidget(artistLabel);
    layout->addLayout(textLayout);

    // Spacer and Play Button
    layout->addStretch();
    playButton = new QPushButton();
    playButton->setIcon(getIcon(":/play.svg"));
    playButton->setToolTip("Play track");
    connect(playButton, &QPushButton::clicked, this, [id]() {
        play(id); // Call the Rust function to play the track
    });
    layout->addWidget(playButton);

    setLayout(layout);
}

TrackItem::~TrackItem()
{
    // Qt will delete child widgets automatically
}