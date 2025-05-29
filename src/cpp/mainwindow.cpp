#include <QFileDialog>
#include <QPainter>
#include <QRadialGradient>
#include <QThread>
#include "mainwindow.h"
#include "trackitem.h"
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
    font.setPointSize(16);
    label->setFont(font);

    verticalLayout_2->addWidget(label, 0, Qt::AlignmentFlag::AlignHCenter|Qt::AlignmentFlag::AlignTop);

    pushButton = new QPushButton(centralwidget);
    pushButton->setObjectName("pushButton");
    pushButton->setText(" Load media");
    pushButton->setIcon(getIcon(":/folder_open.svg"));
    pushButton->setStyleSheet("QPushButton { padding: 8px; border-radius: 4px; border: 1px solid rgba(71,65,75,1); background: rgba(58,51,62,1); }"
                                "QPushButton:hover { background: rgba(58,59,65, 0.8); }"
                                "QPushButton:pressed { background: rgba(48,49,56, 0.8); }");

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

    openMediaDirectoryButton = new QPushButton(centralwidget);
    openMediaDirectoryButton->setObjectName("openMediaDirectoryButton");
    openMediaDirectoryButton->setText(" Open media directory");
    openMediaDirectoryButton->setStyleSheet("QPushButton { padding: 8px; border-radius: 4px; border: 1px solid rgba(71,65,75,1); background: rgba(58,51,62,1); }"
                                "QPushButton:hover { background: rgba(58,59,65, 0.8); }"
                                "QPushButton:pressed { background: rgba(48,49,56, 0.8); }");
    openMediaDirectoryButton->setIcon(getIcon(":/folder_list.svg"));
    connect(openMediaDirectoryButton, &QPushButton::clicked, this, [this]() {
        QString dir = QFileDialog::getExistingDirectory(
            this,
            tr("Open Media Directory"),
            "",
            QFileDialog::ShowDirsOnly | QFileDialog::DontResolveSymlinks
        );
        if (!dir.isEmpty()) {
            // Call the Rust function via cxx bridge (no rust:: namespace)
            open_media_directory(dir.toStdString());
        }
    });

    verticalLayout_2->addWidget(pushButton, 0, Qt::AlignmentFlag::AlignTop);
    verticalLayout_2->addWidget(openMediaDirectoryButton, 0, Qt::AlignmentFlag::AlignTop);

    verticalSpacer = new QSpacerItem(20, 40, QSizePolicy::Policy::Minimum, QSizePolicy::Policy::Expanding);

    verticalLayout_2->addItem(verticalSpacer);


    horizontalLayout->addLayout(verticalLayout_2);
    tabWidget = new QTabWidget();
    tabWidget->setObjectName("tabWidget");
    tabWidget->setStyleSheet("QTabWidget::pane { border: 0px; } QTabBar::tab { background: rgba(30, 30, 30, 0.5); color: white; padding: 8px; border-radius: 8px; } QTabBar::tab:selected { background: rgba(50, 50, 50, 0.5); }");
    verticalLayout_3 = new QVBoxLayout();
    verticalLayout_3->setObjectName("verticalLayout_3");
    trackList = new QListWidget();
    trackList->setObjectName("trackList");
    trackList->setStyleSheet("background: rgba(30, 30, 30, 0.5); color: white; border-radius: 8px;");
    tabWidget->addTab(trackList, "Tracks");
    lyricScrollArea = new QScrollArea(this);
    lyricContainer = new QWidget;
    lyricLayout = new QVBoxLayout(lyricContainer);
    lyricContainer->setLayout(lyricLayout);
    lyricScrollArea->setWidget(lyricContainer);
    lyricScrollArea->setWidgetResizable(true);
    lyricScrollArea->setStyleSheet("background: rgba(30, 30, 30, 0.3); color: white; border-radius: 8px;");
    tabWidget->addTab(lyricScrollArea, "Lyrics");
    verticalLayout_3->addWidget(tabWidget);


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

void MainWindow::paintEvent(QPaintEvent *event)
{
    QPainter painter(this);
    QRect rect = this->rect();
    QRadialGradient gradient(rect.center(), rect.width() * 0.7);
    gradient.setColorAt(0, QColor(31, 0, 28)); // Center color
    gradient.setColorAt(1, QColor(15, 0, 60)); // Edge color
    painter.fillRect(rect, gradient);
    QMainWindow::paintEvent(event);
}

void MainWindow::addTrack(rust::String id, rust::String title, rust::String artists) {
    QListWidgetItem* item = new QListWidgetItem(trackList);
    TrackItem* trackWidget = new TrackItem(std::string(id), QString::fromStdString(std::string(title)), QString::fromStdString(std::string(artists)), "");
    item->setSizeHint(trackWidget->sizeHint());
    trackList->addItem(item);
    trackList->setItemWidget(item, trackWidget);
}

void MainWindow::showEvent(QShowEvent *event)
{
    static bool initialized = false;
    if (!initialized) {
        auto tracks = get_track_list();
        for (const auto& track : tracks) {
            addTrack(track.id, track.title, track.artists);
        }
        initialize_controls();
        initialized = true;
    }
    QMainWindow::showEvent(event);
}

MediaPlayer* MainWindow::getMediaPlayer() {
    return widget;
}

void MainWindow::loadLyrics() {
    if (QThread::currentThread() != this->thread()) {
        QMetaObject::invokeMethod(this, [this]() { loadLyrics(); }, Qt::QueuedConnection);
        return;
    }
    qDeleteAll(lyricLabels);
    lyricLabels.clear();
    QLayoutItem *child;
    while ((child = lyricLayout->takeAt(0)) != nullptr) {
        delete child;
    }
    lyricTimestamps.clear();
    auto lyrics = get_lyrics_for_current_track();
    for (const auto &line : lyrics) {
        QLabel *label = new QLabel(QString::fromStdString(std::string(line.text)));
        label->setStyleSheet("color: gray;");
        lyricLayout->addWidget(label);
        lyricLabels.append(label);
        lyricTimestamps.push_back(line.timestamp);
    }
}

void MainWindow::updateLyricHighlight(double currentTime) {
    if (QThread::currentThread() != this->thread()) {
        QMetaObject::invokeMethod(this, [this, currentTime]() { updateLyricHighlight(currentTime); }, Qt::QueuedConnection);
        return;
    }
    int highlightIndex = 0;
    for (int i = 0; i < lyricTimestamps.size(); ++i) {
        if (lyricTimestamps[i] > currentTime) {
            highlightIndex = (i == 0) ? 0 : i - 1;
            break;
        }
        highlightIndex = i;
    }
    for (int i = 0; i < lyricLabels.size(); ++i) {
        if (i == highlightIndex)
            lyricLabels[i]->setStyleSheet("color: white; font-weight: bold; background-color: transparent; font-size: 16px;");
        else
            lyricLabels[i]->setStyleSheet("color: gray; background-color: transparent; font-size: 14px;");
    }
    if (!lyricLabels.isEmpty()) {
        QWidget *highlighted = lyricLabels[highlightIndex];
        lyricScrollArea->ensureWidgetVisible(highlighted, 100000, 40);
    }
}