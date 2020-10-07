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
package org.astonbitecode.j4rs.api.jfx;

import javafx.application.Application;
import javafx.stage.Stage;
import org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport;

import java.util.concurrent.atomic.AtomicReference;

public class FxApplication extends Application {
    private static AtomicReference<NativeCallbackToRustChannelSupport> callback = new AtomicReference<>(null);

    @Override
    public void start(Stage fxStage) {
        callback.get().doCallback(fxStage);
    }

    static void setCallback(NativeCallbackToRustChannelSupport nativeCallbackToRustChannelSupport) {
        callback.getAndSet(nativeCallbackToRustChannelSupport);
    }
}
