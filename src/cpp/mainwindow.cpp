#include <QFileDialog>
#include "mainwindow.h"
#include "../../target/cxxbridge/vibrance/src/main.rs.h"

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
{
    setupUi();
}

MainWindow::~MainWindow()
{
    // Qt will delete child widgets automatically
}

void MainWindow::setupUi()
{
    if (objectName().isEmpty())
        setObjectName("MainWindow");
    resize(800, 600);
    centralwidget = new QWidget(this);
    centralwidget->setObjectName("centralwidget");
    verticalLayout = new QVBoxLayout(centralwidget);
    verticalLayout->setObjectName("verticalLayout");
    verticalLayout->setContentsMargins(-1, 9, -1, 0);
    horizontalLayout = new QHBoxLayout();
    horizontalLayout->setObjectName("horizontalLayout");
    verticalLayout_2 = new QVBoxLayout();
    verticalLayout_2->setObjectName("verticalLayout_2");
    label = new QLabel(centralwidget);
    label->setObjectName("label");
    label->setText("Vibrance");
    QFont font;
    font.setFamilies({QString::fromUtf8("HONOR Sans")});
    font.setPointSize(16);
    label->setFont(font);

    verticalLayout_2->addWidget(label, 0, Qt::AlignmentFlag::AlignHCenter|Qt::AlignmentFlag::AlignTop);

    pushButton = new QPushButton(centralwidget);
    pushButton->setObjectName("pushButton");
    pushButton->setText("Load media");

    // Connect the button to a lambda that opens a file dialog
    connect(pushButton, &QPushButton::clicked, this, [this]() {
        QString fileName = QFileDialog::getOpenFileName(
            this,
            tr("Open Audio File"),
            "",
            tr("Audio Files (*.mp3 *.wav *.ogg *.flac);;All Files (*)")
        );
        if (!fileName.isEmpty()) {
            // Call the Rust function via cxx bridge (no rust:: namespace)
            process_audio_file(fileName.toStdString());
        }
    });

    verticalLayout_2->addWidget(pushButton, 0, Qt::AlignmentFlag::AlignTop);

    verticalSpacer = new QSpacerItem(20, 40, QSizePolicy::Policy::Minimum, QSizePolicy::Policy::Expanding);

    verticalLayout_2->addItem(verticalSpacer);


    horizontalLayout->addLayout(verticalLayout_2);

    verticalLayout_3 = new QVBoxLayout();
    verticalLayout_3->setObjectName("verticalLayout_3");
    scrollArea = new QScrollArea(centralwidget);
    scrollArea->setObjectName("scrollArea");
    scrollArea->setWidgetResizable(true);
    scrollAreaWidgetContents = new QWidget();
    scrollAreaWidgetContents->setObjectName("scrollAreaWidgetContents");
    scrollAreaWidgetContents->setGeometry(QRect(0, 0, 576, 386));
    listView = new QListView(scrollAreaWidgetContents);
    listView->setObjectName("listView");
    listView->setGeometry(QRect(120, 100, 256, 192));
    scrollArea->setWidget(scrollAreaWidgetContents);

    verticalLayout_3->addWidget(scrollArea);


    horizontalLayout->addLayout(verticalLayout_3);

    horizontalLayout->setStretch(0, 1);
    horizontalLayout->setStretch(1, 3);

    verticalLayout->addLayout(horizontalLayout);

    widget = new MediaPlayer(centralwidget);
    widget->setObjectName("widget");
    widget->setMinimumSize(QSize(0, 150));
    widget->setMaximumSize(QSize(16777215, 150));

    verticalLayout->addWidget(widget, 0, Qt::AlignmentFlag::AlignBottom);

    setCentralWidget(centralwidget);
    menubar = new QMenuBar(this);
    menubar->setObjectName("menubar");
    menubar->setGeometry(QRect(0, 0, 800, 21));
    setMenuBar(menubar);
    statusbar = new QStatusBar(this);
    statusbar->setObjectName("statusbar");
    setStatusBar(statusbar);
}

MediaPlayer* MainWindow::getMediaPlayer() {
    return widget;
}
