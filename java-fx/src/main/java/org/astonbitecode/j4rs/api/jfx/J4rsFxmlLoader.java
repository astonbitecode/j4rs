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

import javafx.fxml.FXMLLoader;
import javafx.scene.Parent;
import javafx.scene.Scene;
import javafx.stage.Stage;
import org.astonbitecode.j4rs.api.jfx.controllers.FxController;
import org.astonbitecode.j4rs.api.jfx.controllers.FxControllerImpl;
import org.astonbitecode.j4rs.api.jfx.errors.FxException;

import java.io.File;
import java.io.IOException;
import java.net.URL;

@SuppressWarnings("unused")
public class J4rsFxmlLoader {
    /**
     * Loads a FXML and returns an {@link FxController} for it.
     *
     * @param stage    The {@link Stage} to load the FXML on.
     * @param fxmlPath The location of the FXML file.
     * @return A {@link FxController} instance.
     * @throws IOException In case that the FXML cannot be loaded.
     * @throws FxException In case the fxml cannot be loaded
     */
    @SuppressWarnings("unused")
    public static FxController loadFxml(Stage stage, String fxmlPath) throws IOException, FxException {
        FXMLLoader loader = new FXMLLoader();
        URL resurl = new File(fxmlPath).toURI().toURL();
        loader.setControllerFactory(clazz -> new FxControllerImpl());
        loader.setLocation(resurl);
        Parent root = loader.load();

        Scene scene = new Scene(root);
        stage.setScene(scene);

        FxController controller = loader.getController();
        if (controller == null) {
            throw new FxException(String.format(
                    "Could not load the fxml %s. Please make sure that its root element contains fx:controller=\"org.astonbitecode.j4rs.api.jfx.controllers.FxController\"",
                    fxmlPath));
        }
        controller.setScene(scene);

        return controller;
    }
}
