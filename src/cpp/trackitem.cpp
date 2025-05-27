#include "trackitem.h"
#include "../../target/cxxbridge/vibrance/src/main.rs.h"

TrackItem::TrackItem(std::string id, const QString &title, const QString &artist, const QString &albumArtPath, QWidget *parent)
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
    titleLabel->setStyleSheet("font-weight: bold; color: white; background-color: transparent;");
    artistLabel->setStyleSheet("color: gray; background-color: transparent;");
    textLayout->addWidget(titleLabel);
    textLayout->addWidget(artistLabel);
    layout->addLayout(textLayout);

    // Spacer and Play Button
    layout->addStretch();
    playButton = new QPushButton("Play");
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