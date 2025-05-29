#include "navigationitem.h"
#include "../../target/cxxbridge/vibrance/src/main.rs.h"
#include "mediaplayer.h"

NavigationItem::NavigationItem(const char* name, const char* icon, QWidget *parent)
    : QWidget(parent)
{
    setFixedHeight(40);
    QHBoxLayout* homeLayout = new QHBoxLayout();
    homeLayout->setContentsMargins(0, 0, 0, 0);
    indicator = new QFrame();
    indicator->setFixedWidth(4);
    indicator->setFixedHeight(20);
    indicator->setStyleSheet("background-color: transparent;");
    homeLayout->addWidget(indicator);
    QLabel* iconLabel = new QLabel();
    iconLabel->setPixmap(getIcon(icon).pixmap(QSize(20, 20)));
    homeLayout->addWidget(iconLabel);
    QLabel* homeLabel = new QLabel(name);
    homeLabel->setStyleSheet("color: white; font-size: 14px;");
    homeLayout->setAlignment(Qt::AlignmentFlag::AlignLeft);
    homeLayout->addWidget(homeLabel);
    setLayout(homeLayout);    
}

NavigationItem::~NavigationItem()
{
    // Qt will delete child widgets automatically
}