#include "volumeflyout.h"
#include "../../target/cxxbridge/vibrance/src/main.rs.h"

VolumeFlyout::VolumeFlyout(QWidget *parent)
    : QWidget(parent)
{
    setupUi();
}

VolumeFlyout::~VolumeFlyout()
{
    // Qt will delete child widgets automatically
}

void VolumeFlyout::setupUi()
{
    
    setFixedSize(40, 120);

    QVBoxLayout *layout = new QVBoxLayout(this);
    slider = new QSlider(Qt::Vertical);
    slider->setRange(0, 100);
    slider->setValue(get_initial_volume()); 
    layout->addWidget(slider);

    connect(slider, &QSlider::valueChanged, this, &VolumeFlyout::volumeChanged);
}