#ifndef TRACKITEM_H
#define TRACKITEM_H

#include <QApplication>
#include <QWidget>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QPushButton>
#include <QDebug>
#include <QLabel>
#include "mediaplayer.h"

QPixmap getAlbumArtPixmap(QByteArray base64ImageData, int size);

class TrackItem : public QWidget
{
    Q_OBJECT

public:
    TrackItem(std::string id, const QString &title, const QString &artist, std::string albumArt, QWidget *parent = nullptr);
    ~TrackItem();

private:
    // UI elements
    QHBoxLayout *layout;
    QLabel *albumArt;
    QVBoxLayout *textLayout;
    QLabel *titleLabel;
    QLabel *artistLabel;
    QPushButton *playButton;
    QFrame *albumArtFrame;
};

#endif // MEDIAPLAYER_H
