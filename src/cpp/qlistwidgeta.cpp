#include "qlistwidgeta.h"

void QListWidgetA::mousePressEvent(QMouseEvent *event){
    if (event->button() == Qt::LeftButton)
        mousePressed = true;
    QListWidget::mousePressEvent(event);
}

void QListWidgetA::mouseMoveEvent(QMouseEvent *event){
    if (!mousePressed)
        QListWidget::mouseMoveEvent(event);
}

void QListWidgetA::mouseReleaseEvent(QMouseEvent *event){
    if (event->button() == Qt::LeftButton)
        mousePressed = false;
    QListWidget::mouseReleaseEvent(event);
}