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
package org.astonbitecode.j4rs.api.jfx.handlers;

import javafx.event.Event;
import javafx.event.EventHandler;
import org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport;

public class J4rsEventHandler<T extends Event> extends NativeCallbackToRustChannelSupport implements EventHandler<T> {
    public J4rsEventHandler() {
        System.out.println("------NEW---");
    }

    @Override
    public void handle(T event) {
        System.out.println("---------");
        doCallback(event);
    }
}
