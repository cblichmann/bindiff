package com.google.security.zynamics.bindiff.gui.tabpanels.viewtabpanel.popupmenus;

import com.google.common.base.Preconditions;
import com.google.security.zynamics.bindiff.enums.ESide;
import com.google.security.zynamics.bindiff.graph.BinDiffGraph;
import com.google.security.zynamics.bindiff.graph.nodes.CombinedDiffNode;
import com.google.security.zynamics.bindiff.graph.nodes.SingleDiffNode;
import com.google.security.zynamics.bindiff.gui.tabpanels.projecttabpanel.actions.OpenFlowGraphsViewAction;
import com.google.security.zynamics.bindiff.gui.tabpanels.viewtabpanel.ViewTabPanelFunctions;
import com.google.security.zynamics.bindiff.gui.tabpanels.viewtabpanel.actions.CopyFunctionAddressAction;
import com.google.security.zynamics.bindiff.gui.tabpanels.viewtabpanel.actions.CopyFunctionNameAction;
import com.google.security.zynamics.bindiff.gui.tabpanels.viewtabpanel.actions.ZoomToNodeAction;
import com.google.security.zynamics.zylib.yfileswrap.gui.zygraph.nodes.ZyGraphNode;
import javax.swing.JMenuItem;
import javax.swing.JPopupMenu;
import javax.swing.JSeparator;

public class CallGraphPopupMenu extends JPopupMenu {
  public CallGraphPopupMenu(
      final ViewTabPanelFunctions controller,
      final BinDiffGraph<?, ?> graph,
      final ZyGraphNode<?> node) {
    Preconditions.checkNotNull(controller);
    Preconditions.checkNotNull(graph);
    Preconditions.checkNotNull(node);

    final JMenuItem openFlowgraphViewsMenuItem =
        new JMenuItem(new OpenFlowGraphsViewAction(controller, node));
    final JMenuItem zoomToFunctionNodeItem = new JMenuItem(new ZoomToNodeAction(graph, node));

    add(openFlowgraphViewsMenuItem);
    add(new JSeparator());
    add(zoomToFunctionNodeItem);
    add(new JSeparator());

    if (node instanceof CombinedDiffNode) {
      final CombinedDiffNode combinedNode = (CombinedDiffNode) node;
      if (combinedNode.getPrimaryDiffNode() != null) {
        final JMenuItem copyPrimaryFunctionAddressItem =
            new JMenuItem(new CopyFunctionAddressAction(combinedNode, ESide.PRIMARY));
        final JMenuItem copyPrimaryFunctionNameItem =
            new JMenuItem(new CopyFunctionNameAction(combinedNode, ESide.PRIMARY));
        add(copyPrimaryFunctionAddressItem);
        add(copyPrimaryFunctionNameItem);
      }
      if (combinedNode.getSecondaryDiffNode() != null) {
        final JMenuItem copySecondaryFunctionAddressItem =
            new JMenuItem(new CopyFunctionAddressAction(combinedNode, ESide.SECONDARY));
        final JMenuItem copySecondaryFunctionNameItem =
            new JMenuItem(new CopyFunctionNameAction(combinedNode, ESide.SECONDARY));
        add(copySecondaryFunctionAddressItem);
        add(copySecondaryFunctionNameItem);
      }
    } else if (node instanceof SingleDiffNode) {
      final JMenuItem copyFunctionAddressItem =
          new JMenuItem(new CopyFunctionAddressAction((SingleDiffNode) node));
      final JMenuItem copyFunctioNameItem =
          new JMenuItem(new CopyFunctionNameAction((SingleDiffNode) node));

      add(copyFunctionAddressItem);
      add(copyFunctioNameItem);
    }
  }
}