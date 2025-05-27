#include "trackitem.h"

TrackItem::TrackItem(const QString &title, const QString &artist, const QString &albumArtPath, QWidget *parent)
    : QWidget(parent)
{
    layout = new QHBoxLayout(this);

    // Album Art
    albumArt = new QLabel;
    QPixmap pixmap(albumArtPath);
    albumArt->setPixmap(pixmap.scaled(50, 50, Qt::KeepAspectRatio, Qt::SmoothTransformation));
    layout->addWidget(albumArt);

    // Title and Artist
    textLayout = new QVBoxLayout;
    titleLabel = new QLabel(title);
    artistLabel = new QLabel(artist);
    artistLabel->setStyleSheet("color: gray;");
    textLayout->addWidget(titleLabel);
    textLayout->addWidget(artistLabel);
    layout->addLayout(textLayout);

    // Spacer and Play Button
    layout->addStretch();
    playButton = new QPushButton("play");
    layout->addWidget(playButton);

    setLayout(layout);
}

TrackItem::~TrackItem()
{
    // Qt will delete child widgets automatically
}