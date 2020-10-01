/*
 * Copyright 2020 astonbitecode
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
package org.astonbitecode.j4rs.api.jfx.controllers;

import javafx.scene.Node;
import javafx.scene.Scene;
import org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport;
import org.astonbitecode.j4rs.api.jfx.errors.ComponentNotFoundException;

import java.net.URL;
import java.util.ResourceBundle;

public class FxControllerImpl implements FxController {
    private Scene scene;

    @Override
    public void initialize(URL url, ResourceBundle resourceBundle) {

    }

    @Override
    public void addControllerInitializedCallback(NativeCallbackToRustChannelSupport callback) {

    }

    @Override
    public void addActionEventCallback(String id, NativeCallbackToRustChannelSupport callback) {

    }

    @Override
    public Node getNodeById(String id) throws ComponentNotFoundException {
        if (scene != null) {
            Node node = scene.lookup(id);
            if (node != null) {
                return node;
            }
        }
        throw new ComponentNotFoundException(String.format("Node with id %s", id));
    }

    @Override
    public void setScene(Scene scene) {
        this.scene = scene;
    }
}
