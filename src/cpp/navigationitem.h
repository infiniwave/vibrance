#ifndef NAVIGATIONITEM_H
#define NAVIGATIONITEM_H

#include <QApplication>
#include <QWidget>
#include <QToolButton>
#include <QSlider>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QPoint>
#include <QDebug>
#include <QFrame>
#include <QLabel>

class NavigationItem : public QWidget
{
    Q_OBJECT

public:
    NavigationItem(const char* name, const char* icon, QWidget *parent = nullptr);
    ~NavigationItem();

    QSize sizeHint() const override {
        return QSize(0, 40);
    }

private:
    // UI elements
    QFrame* indicator;

public slots:
    void setActive(bool active) {
        if (active) {
            indicator->setStyleSheet("background-color: rgba(208,159,223, 1); border-radius: 2px;");
        } else {
            indicator->setStyleSheet("background-color: transparent;");
        }
    }
};

#endif // MEDIAPLAYER_H
