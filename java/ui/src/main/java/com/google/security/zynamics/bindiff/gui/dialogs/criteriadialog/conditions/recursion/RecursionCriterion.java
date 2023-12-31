// Copyright 2011-2023 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package com.google.security.zynamics.bindiff.gui.dialogs.criteriadialog.conditions.recursion;

import com.google.security.zynamics.bindiff.gui.dialogs.criteriadialog.conditions.ConditionCriterion;
import com.google.security.zynamics.bindiff.utils.ResourceUtils;
import com.google.security.zynamics.zylib.gui.zygraph.nodes.CViewNode;
import com.google.security.zynamics.zylib.yfileswrap.gui.zygraph.nodes.ZyGraphNode;
import javax.swing.Icon;
import javax.swing.ImageIcon;
import javax.swing.JPanel;

public class RecursionCriterion extends ConditionCriterion {
  private static final ImageIcon RECURSION_CONDITION_ICON =
      ResourceUtils.getImageIcon("data/selectbycriteriaicons/recursion-condition.png");

  private final RecursionCriterionPanel panel = new RecursionCriterionPanel(this);

  @Override
  public String getCriterionDescription() {
    return String.format(
        "Nodes with %s Recursion",
        panel.getRecursionState() == RecursionState.IS_RECURSION ? "" : "no");
  }

  @Override
  public JPanel getCriterionPanel() {
    return panel;
  }

  @Override
  public Icon getIcon() {
    return RECURSION_CONDITION_ICON;
  }

  @Override
  public boolean matches(final ZyGraphNode<? extends CViewNode<?>> node) {
    if (panel.getRecursionState() == RecursionState.IS_RECURSION) {
      return node.getChildren().contains(node);
    } else {
      return node.getChildren().contains(node);
    }
  }

  public void update() {
    notifyListeners();
  }
}
