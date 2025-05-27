#include "volumeflyout.h"

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
    slider->setValue(50);
    layout->addWidget(slider);

    connect(slider, &QSlider::valueChanged, this, &VolumeFlyout::volumeChanged);
}

void VolumeFlyout::initializeVolume(int initialVolume)
{
    slider->setValue(initialVolume);
}