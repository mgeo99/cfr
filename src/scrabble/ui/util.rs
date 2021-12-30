use glib::IsA;
use gtk::{
    Adjustment,
    CellLayoutExt,
    ContainerExt,
    ScrolledWindow,
    TreeViewColumn,
    TreeViewColumnExt,
    TreeViewExt,
    Widget,
};

pub fn append_column(
    title: &str,
    v: &mut Vec<gtk::TreeViewColumn>,
    treeview: &gtk::TreeView,
    max_width: Option<i32>,
) -> i32 {
    let id = v.len() as i32;
    let renderer = gtk::CellRendererText::new();

    let column = TreeViewColumn::new();
    column.set_title(title);
    if let Some(max_width) = max_width {
        column.set_max_width(max_width);
        column.set_expand(true);
    }
    column.set_min_width(10);
    column.pack_start(&renderer, true);
    column.add_attribute(&renderer, "text", id);
    column.set_clickable(true);
    column.set_sort_column_id(id); // todo
    column.set_resizable(true);
    treeview.append_column(&column);
    v.push(column);

    id
}

pub fn create_scroll_window<T>(window: &T) -> ScrolledWindow
where
    T: IsA<Widget>,
{
    let no_adjustment: Option<Adjustment> = None;
    let scroll: Option<Adjustment> = Some(Adjustment::new(
        0.0,
        std::f64::MIN,
        std::f64::MAX,
        1.0,
        0.0,
        0.0,
    ));
    let container = ScrolledWindow::new(no_adjustment.as_ref(), scroll.as_ref());
    container.add(window);
    container
}
