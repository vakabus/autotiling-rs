use swayipc::{Connection, EventType, Node};
use swayipc::{Event, NodeLayout, NodeType, WindowChange};

use clap::Parser;

fn switch_splitting(conn: &mut Connection, ratio: f32) -> Result<(), String> {
    // get info from focused node and parent node which unfortunately requires us to call get_tree
    let tree = conn.get_tree().map_err(|_| "get_tree() failed")?;
    let focused_node = tree
        .find_focused_as_ref(|n| n.focused)
        .ok_or("Could not find the focused node")?;
    let parent = tree
        .find_focused_as_ref(|n| n.nodes.iter().any(|n| n.focused))
        .ok_or("No parent")?;

    // check for special cases when we should not do anything
    if should_we_ignore_this_window(focused_node) {
        return Ok(());
    }

    // if there is a single window in the workspace, always split horizontaly
    if parent.node_type == NodeType::Workspace && parent.nodes.len() == 1 {
        configure_layout(NodeLayout::SplitH, parent, conn);
        return Ok(());
    }

    let real_ratio = (focused_node.rect.height as f32) / (focused_node.rect.width as f32);
    if real_ratio > ratio {
        configure_layout(NodeLayout::SplitV, parent, conn);
    } else {
        configure_layout(NodeLayout::SplitH, parent, conn);
    }

    Ok(())
}

/**
 * Determine, whether we should do anything with this window
 */
fn should_we_ignore_this_window(focused_node: &swayipc::Node) -> bool {
    // get info from the focused child node
    let is_stacked = focused_node.layout == NodeLayout::Stacked;
    let is_tabbed = focused_node.layout == NodeLayout::Tabbed;
    let is_floating = focused_node.node_type == NodeType::FloatingCon;
    let is_full_screen = focused_node.percent.unwrap_or(1.0) > 1.0;

    is_floating || is_full_screen || is_stacked || is_tabbed
}

/**
 * Configure layout of the currently focused container
 */
fn configure_layout(new_layout: NodeLayout, parent: &Node, conn: &mut Connection) {
    // as an optimization, skip IPC if parent already set correctly
    if new_layout == parent.layout {
        return;
    }

    let cmd = match new_layout {
        NodeLayout::SplitV => "splitv",
        NodeLayout::SplitH => "splith",
        _ => "nop",
    };
    conn.run_command(cmd).unwrap();
}

#[derive(Parser)]
#[clap(version, author, about)]
struct Cli {
    /// Activate autotiling only on this workspace. More than one workspace may be specified.
    #[clap(long, short = 'w')]
    workspace: Vec<i32>,

    #[clap(long, short = 'r', default_value_t = 0.4)]
    ratio: f32,
}

fn main() -> Result<(), std::io::Error> {
    let args = Cli::parse();

    let mut conn = Connection::new().unwrap();
    for event in Connection::new()
        .unwrap()
        .subscribe(&[EventType::Window])
        .unwrap()
    {
        match event.unwrap() {
            Event::Window(e) => {
                if let WindowChange::Focus = e.change {
                    // We can not use the e.container because the data is stale.
                    // If we compare that node data with the node given from get_tree() after we
                    // delete a node we find that the e.container.rect.height and e.container.rect.width are stale,
                    // and therefore we make the wrong decision on which layout our next window should be.
                    // Refer to https://github.com/swaywm/sway/issues/5873
                    if let Err(err) = switch_splitting(&mut conn, args.ratio) {
                        eprintln!("err: {}", err);
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}
