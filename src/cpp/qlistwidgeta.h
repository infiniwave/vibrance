#ifndef QLISTWIDGETA_H
#define QLISTWIDGETA_H
#include <QListWidget>
#include <QMouseEvent>

// QListWidgetA: because QListWidget is dumb and switches selection on drag :)
class QListWidgetA: public QListWidget
{ 
private:
    bool mousePressed;
public:
    QListWidgetA(QWidget *parent = nullptr): QListWidget(parent), mousePressed(false) {}
protected:
    virtual void mousePressEvent(QMouseEvent *event);
    virtual void mouseMoveEvent(QMouseEvent *event);
    virtual void mouseReleaseEvent(QMouseEvent *event);
};

#endif