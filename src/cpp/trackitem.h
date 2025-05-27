#ifndef TRACKITEM_H
#define TRACKITEM_H

#include <QApplication>
#include <QWidget>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QPushButton>
#include <QDebug>
#include <QLabel>

class TrackItem : public QWidget
{
    Q_OBJECT

public:
    TrackItem(const QString &title, const QString &artist, const QString &albumArtPath, QWidget *parent = nullptr);
    ~TrackItem();

private:
    // UI elements
    QHBoxLayout *layout;
    QLabel *albumArt;
    QVBoxLayout *textLayout;
    QLabel *titleLabel;
    QLabel *artistLabel;
    QPushButton *playButton;
};

#endif // MEDIAPLAYER_H
