#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QtCore/QVariant>
#include <QMainWindow>
#include <QtWidgets/QApplication>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMainWindow>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpacerItem>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>
#include <QScrollArea>
#include <QVector>
#include <vector>
#include "mediaplayer.h"
#include "../../target/cxxbridge/vibrance/src/main.rs.h"
#include "qlistwidgeta.h"
#include "navigationitem.h"

class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    MainWindow(QWidget *parent = nullptr);
    ~MainWindow();

    MediaPlayer* getMediaPlayer();
    void addTrack(rust::String id, rust::String title, rust::String artists, rust::String albumArt);

protected:
    void paintEvent(QPaintEvent *event) override;
    void showEvent(QShowEvent *event) override;

private:
    // UI elements
    QWidget *centralwidget;
    QVBoxLayout *verticalLayout;
    QHBoxLayout *horizontalLayout;
    QVBoxLayout *verticalLayout_2;
    QLabel *label;
    QPushButton *pushButton;
    QPushButton *openMediaDirectoryButton;
    QSpacerItem *verticalSpacer;
    QVBoxLayout *verticalLayout_3;
    QListWidget *trackList;
    MediaPlayer *widget;
    QMenuBar *menubar;
    QStatusBar *statusbar;

    QScrollArea *lyricScrollArea;
    QWidget *lyricContainer;
    QVBoxLayout *lyricLayout;
    QVector<QLabel*> lyricLabels;
    QTimer *lyricScrollTimer;
    QTabWidget *tabWidget;
    std::vector<double> lyricTimestamps;
    void setupUi();

    NavigationItem* homeItemWidget;
    NavigationItem* libraryItemWidget;
    NavigationItem* settingsItemWidget;
    NavigationItem* searchItemWidget;

public slots:
    void loadLyrics();
    void updateLyricHighlight(double currentTime);
};

#endif // MEDIAPLAYER_H
