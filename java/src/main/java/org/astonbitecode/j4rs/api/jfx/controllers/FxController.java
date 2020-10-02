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

import javafx.event.Event;
import javafx.event.EventHandler;
import javafx.event.EventType;
import javafx.fxml.Initializable;
import javafx.scene.Node;
import javafx.scene.Scene;
import org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport;
import org.astonbitecode.j4rs.api.jfx.errors.ComponentNotFoundException;

public interface FxController extends Initializable {
    /**
     * This will be called when the initialize method of {@link Initializable} is called.
     *
     * @param callback The callback to add.
     */
    void addControllerInitializedCallback(NativeCallbackToRustChannelSupport callback);

    /**
     * Add a handler for an {@link javafx.event.ActionEvent} that comes from a component with a specific id.
     *
     * @param id        The id of the callback.
     * @param handler   The handler to add.
     * @param eventType The EventType for Event to handle.
     */
    void addEventHandler(String id, EventHandler<Event> handler, EventType<?> eventType) throws ComponentNotFoundException;

    /**
     * Retrieves a node given its ID.
     *
     * @param id The id of the node to retrieve.
     * @return The {@link Node} found.
     * @throws ComponentNotFoundException In case that the node is not found.
     */
    Node getNodeById(String id) throws ComponentNotFoundException;

    /**
     * Sets a scene for this controller.
     *
     * @param scene The scene to set.
     */
    void setScene(Scene scene);
}
