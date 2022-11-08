// Copyright 2020 astonbitecode
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use std::env;
use std::convert::{TryFrom, TryInto};
use std::path::PathBuf;

use crate::{InvocationArg, Jvm, MavenArtifact};
use crate::api::{CLASS_J4RS_EVENT_HANDLER, CLASS_J4RS_FXML_LOADER, CLASS_NATIVE_CALLBACK_TO_RUST_CHANNEL_SUPPORT};
use crate::api::instance::{Instance, InstanceReceiver};
use crate::errors;
use crate::errors::{J4RsError, opt_to_res};

/// Provides JavaFx support.
pub trait JavaFxSupport {
    /// Triggers the start of a JavaFX application.
    /// When the JavaFX application starts, the `InstanceReceiver` channel will receive an Instance of `javafx.stage.Stage`.
    ///
    /// The UI may start being built using the provided `Stage`
    fn start_javafx_app(&self) -> errors::Result<InstanceReceiver>;
    /// Deploys the required dependencies to run a JavaFX application in order to be able to be used by j4rs.
    fn deploy_javafx_dependencies(&self) -> errors::Result<()>;
    /// Creates an instance receiver that will be receiving `Instance`s of events.
    /// The fx_event_type argument is the type of the event that we want to handle and receive Instances for.
    ///
    /// For example, to create an `InstanceReceiver` for a 'javafx.scene.control.Button',
    /// you need to call the method by using the button as the _instance_ argument
    /// `FxEventType::ActionEvent_Action` as the fx_event_type argument
    fn get_javafx_event_receiver(&self, instance: &Instance, fx_event_type: FxEventType) -> errors::Result<InstanceReceiver>;
    /// Creates an instance receiver that will be receiving `Instance`s of events for onclose requests of a `Stage`.
    ///
    /// The instance passed as argument needs to be of class `javafx.stage.Stage`.
    fn on_close_event_receiver(&self, stage: &Instance) -> errors::Result<InstanceReceiver>;
    /// Loads a FXML and returns a Result of a FxController for it.
    fn load_fxml(&self, path: &PathBuf, stage: &Instance) -> errors::Result<FxController>;
}

impl JavaFxSupport for Jvm {
    /// Triggers the start of a JavaFX application.
    /// When the JavaFX application starts, the `InstanceReceiver` channel will receive an Instance of `javafx.stage.Stage`.
    ///
    /// The UI may start being built using the provided `Stage`
    fn start_javafx_app(&self) -> errors::Result<InstanceReceiver> {
        let fx_callback = self.create_instance(
            "org.astonbitecode.j4rs.api.jfx.FxApplicationStartCallback",
            &[])?;

        self.invoke_to_channel(&fx_callback, "setCallbackToApplicationAndLaunch", &[])
    }

    /// Creates an instance receiver that will be receiving `Instance`s of events.
    /// The fx_event_type argument is the type of the event that we want to handle and receive Instances for.
    ///
    /// For example, to create an `InstanceReceiver` for a 'javafx.scene.control.Button',
    /// you need to call the method by using the button as the _instance_ argument
    /// `FxEventType::ActionEvent_Action` as the fx_event_type argument
    fn get_javafx_event_receiver(&self, instance: &Instance, fx_event_type: FxEventType) -> errors::Result<InstanceReceiver> {
        let j4rs_event_handler = self.create_instance(CLASS_J4RS_EVENT_HANDLER, &[])?;
        let btn_action_channel = self.init_callback_channel(&j4rs_event_handler)?;

        let (event_class, field) = fx_event_type_to_event_class_and_field(fx_event_type);
        let event_type_instance = self.static_class_field(&event_class, &field)?;

        self.invoke(&instance, "addEventHandler", &[event_type_instance.try_into()?, j4rs_event_handler.try_into()?])?;
        Ok(btn_action_channel)
    }

    /// Creates an instance receiver that will be receiving `Instance`s of events for onclose requests of a `Stage`.
    ///
    /// The instance passed as argument needs to be of class `javafx.stage.Stage`.
    fn on_close_event_receiver(&self, stage: &Instance) -> errors::Result<InstanceReceiver> {
        let j4rs_event_handler = self.create_instance(CLASS_J4RS_EVENT_HANDLER, &[])?;
        let action_channel = self.init_callback_channel(&j4rs_event_handler)?;
        self.invoke(&stage, "setOnCloseRequest", &[InvocationArg::try_from(j4rs_event_handler)?])?;
        Ok(action_channel)
    }

    /// Deploys the required dependencies to run a JavaFX application in order to be able to be used by j4rs.
    fn deploy_javafx_dependencies(&self) -> errors::Result<()> {
        let target_os_res = env::var("CARGO_CFG_TARGET_OS");
        if target_os_res.is_ok() {
            let target_os = target_os_res.as_ref().map(|x| &**x).unwrap_or("unknown");
            if target_os == "android" {
                return Ok(());
            }

            println!("cargo:warning=javafx dependencies deployment...");
            maven("org.openjfx:javafx-base:13.0.2", self);
            maven(&format!("org.openjfx:javafx-base:13.0.2:{}", target_os), self);
            maven("org.openjfx:javafx-controls:13.0.2", self);
            maven(&format!("org.openjfx:javafx-controls:13.0.2:{}", target_os), self);
            maven("org.openjfx:javafx-fxml:13.0.2", self);
            maven(&format!("org.openjfx:javafx-fxml:13.0.2:{}", target_os), self);
            maven("org.openjfx:javafx-graphics:13.0.2", self);
            maven(&format!("org.openjfx:javafx-graphics:13.0.2:{}", target_os), self);
            maven("org.openjfx:javafx-media:13.0.2", self);
            maven(&format!("org.openjfx:javafx-media:13.0.2:{}", target_os), self);
            println!("cargo:warning=javafx dependencies deployment completed...");

            Ok(())
        } else {
            Err(J4RsError::GeneralError("deploy_javafx_dependencies can be used only during build time. It should be called by a build script.".to_string()))
        }
    }

    fn load_fxml(&self, path: &PathBuf, stage: &Instance) -> errors::Result<FxController> {
        let cloned = self.clone_instance(&stage)?;
        let path_str = opt_to_res(path.to_str())?;
        let controller = self.invoke_static(
            CLASS_J4RS_FXML_LOADER,
            "loadFxml",
            &[cloned.try_into()?, path_str.try_into()?])?;
        Ok(FxController::new(controller))
    }
}

fn maven(s: &str, jvm: &Jvm) {
    let artifact = MavenArtifact::from(s);
    let _ = jvm.deploy_artifact(&artifact).map_err(|error| {
        println!("cargo:warning=Could not download Maven artifact {}: {:?}", s, error);
    });
}

pub struct FxController {
    controller: Instance
}

impl FxController {
    fn new(controller: Instance) -> FxController {
        FxController { controller }
    }

    /// Creates an InstanceReceiver that will receive an Event once the Controller in initialized by the JavaFX framework.
    ///
    /// JavaFX FXMLLoader will automatically do the call after the root element of the controller has been completely processed.
    pub fn on_initialized_callback(&self, jvm: &Jvm) -> errors::Result<InstanceReceiver> {
        let channel_support = jvm.create_instance(CLASS_NATIVE_CALLBACK_TO_RUST_CHANNEL_SUPPORT, &[])?;
        let instance_receiver = jvm.init_callback_channel(&channel_support);
        jvm.invoke(&self.controller, "addControllerInitializedCallback", &[channel_support.try_into()?])?;
        instance_receiver
    }

    /// Returns an InstanceReceiver that receives events of etype fx_event_type from the JavaFX node with the specified node_id (id attribute of the fxml element).
    pub fn get_event_receiver_for_node(&self, node_id: &str, fx_event_type: FxEventType, jvm: &Jvm) -> errors::Result<InstanceReceiver> {
        let j4rs_event_handler = jvm.create_instance(CLASS_J4RS_EVENT_HANDLER, &[])?;
        let event_channel = jvm.init_callback_channel(&j4rs_event_handler)?;
        let (event_class, field) = fx_event_type_to_event_class_and_field(fx_event_type);
        let event_type_instance = jvm.static_class_field(&event_class, &field)?;
        jvm.invoke(&self.controller, "addEventHandler", &[node_id.try_into()?, j4rs_event_handler.try_into()?, event_type_instance.try_into()?])?;
        Ok(event_channel)
    }
}

#[allow(non_camel_case_types)]
/// Types of FX events.
pub enum FxEventType {
    DirectEvent_Any,
    DirectEvent_Direct,
    RedirectedEvent_Any,
    RedirectedEvent_Redirected,
    FocusUngrabEvent_Any,
    FocusUngrabEvent_FocusUngrub,
    WorkerStateEvent_Any,
    WorkerStateEvent_WorkerStateCancelled,
    WorkerStateEvent_WorkerStateFailed,
    WorkerStateEvent_WorkerStateReady,
    WorkerStateEvent_WorkerStateRunning,
    WorkerStateEvent_WorkerStateScheduled,
    WorkerStateEvent_WorkerStateSucceeded,
    ActionEvent_Any,
    ActionEvent_Action,
    CheckboxTreeItem_TreeModificationEvent_Any,
    DialogEvent_Any,
    DialogEvent_DialogCloseRequest,
    DialogEvent_DialogHidden,
    DialogEvent_DialogHiding,
    DialogEvent_DialogShowing,
    DialogEvent_DialogShown,
    Listview_EditEvent_Any,
    ScrollToEvent_Any,
    SortEvent_Any,
    TableColumn_CellEditEvent_Any,
    TreeItem_TreeModificationEvent_Any,
    TreeTableView_EditEvent_Any,
    TreeView_EditEvent_Any,
    ContextMenuEvent_Any,
    ContextMenuEvent_ContextMenuRequested,
    DragEvent_Any,
    DragEvent_DragDone,
    DragEvent_DragDropped,
    DragEvent_DragEntered,
    DragEvent_DragEnteredTarget,
    DragEvent_DragExited,
    DragEvent_DragExitedTarget,
    DragEvent_DragOver,
    GestureEvent_Any,
    InputEvent_Any,
    InputMethodEvent_Any,
    InputMethodEvent_InputMethodTextChanged,
    KeyEvent_Any,
    KeyEvent_KeyPressed,
    KeyEvent_KeyReleased,
    KeyEvent_KeyTyped,
    MouseDragEvent_Any,
    MouseDragEvent_DragDetected,
    MouseDragEvent_MouseClicked,
    MouseDragEvent_MouseDragEntered,
    MouseDragEvent_MouseDragEnteredTarget,
    MouseDragEvent_MouseDragExited,
    MouseDragEvent_MouseDragExitedTarget,
    MouseDragEvent_MouseDragOver,
    MouseDragEvent_MouseDragReleased,
    MouseDragEvent_MouseDragged,
    MouseDragEvent_MouseEntered,
    MouseDragEvent_MouseEnteredTarget,
    MouseDragEvent_MouseExited,
    MouseDragEvent_MouseExitedTarget,
    MouseDragEvent_MouseMoved,
    MouseDragEvent_MousePressed,
    MouseDragEvent_MouseReleased,
    MouseEvent_Any,
    MouseEvent_DragDetected,
    MouseEvent_MouseClicked,
    MouseEvent_MouseDragged,
    MouseEvent_MouseEntered,
    MouseEvent_MouseEnteredTarget,
    MouseEvent_MouseExited,
    MouseEvent_MouseExitedTarget,
    MouseEvent_MouseMoved,
    MouseEvent_MousePressed,
    MouseEvent_MouseReleased,
    RotateEvent_Any,
    RotateEvent_Rotate,
    RotateEvent_RotationFinished,
    RotateEvent_RotationStarted,
    ScrollEvent_Any,
    ScrollEvent_Scroll,
    ScrollEvent_ScrollFinished,
    ScrollEvent_ScrollStarted,
    SwipeEvent_Any,
    SwipeEvent_SwipeDown,
    SwipeEvent_SwipeLeft,
    SwipeEvent_SwipeRight,
    SwipeEvent_SwipeUp,
    TouchEvent_Any,
    TouchEvent_TouchMoved,
    TouchEvent_TouchPressed,
    TouchEvent_TouchReleased,
    TouchEvent_TouchStationary,
    ZoomEvent_Any,
    ZoomEvent_Zoom,
    ZoomEvent_ZoomFinished,
    ZoomEvent_ZoomStarted,
    MediaMediaErrorEvent_Any,
    MediaMediaErrorEvent_MediaError,
    MediaMediaMarkerEvent_Action,
    MediaMediaMarkerEvent_Any,
    TransformChangedEvent_Any,
    TransformChangedEvent_TransformChanged,
    WindowEvent_Any,
    WindowEvent_WindowCloseRequest,
    WindowEvent_WindowHidden,
    WindowEvent_WindowHiding,
    WindowEvent_WindowShowing,
    WindowEvent_WindowShown,
}

fn fx_event_type_to_event_class_and_field(event_type: FxEventType) -> (String, String) {
    let (class, field) = match event_type {
        FxEventType::DirectEvent_Any => ("com.sun.javafx.event.DirectEvent", "ANY"),
        FxEventType::DirectEvent_Direct => ("com.sun.javafx.event.DirectEvent", "DIRECT"),
        FxEventType::RedirectedEvent_Any => ("com.sun.javafx.event.RedirectedEvent", "ANY"),
        FxEventType::RedirectedEvent_Redirected => ("com.sun.javafx.event.RedirectedEvent", "REDIRECTED"),
        FxEventType::FocusUngrabEvent_Any => ("com.sun.javafx.stage.FocusUngrabEvent", "ANY"),
        FxEventType::FocusUngrabEvent_FocusUngrub => ("com.sun.javafx.stage.FocusUngrabEvent", "FOCUS_UNGRUB"),
        FxEventType::WorkerStateEvent_Any => ("javafx.concurrent.WorkerStateEvent", "ANY"),
        FxEventType::WorkerStateEvent_WorkerStateCancelled => ("javafx.concurrent.WorkerStateEvent", "WORKER_STATE_CANCELLED"),
        FxEventType::WorkerStateEvent_WorkerStateFailed => ("javafx.concurrent.WorkerStateEvent", "WORKER_STATE_FAILED"),
        FxEventType::WorkerStateEvent_WorkerStateReady => ("javafx.concurrent.WorkerStateEvent", "WORKER_STATE_READY"),
        FxEventType::WorkerStateEvent_WorkerStateRunning => ("javafx.concurrent.WorkerStateEvent", "WORKER_STATE_RUNNING"),
        FxEventType::WorkerStateEvent_WorkerStateSucceeded => ("javafx.concurrent.WorkerStateEvent", "WORKER_STATE_SUCCEEDED"),
        FxEventType::WorkerStateEvent_WorkerStateScheduled => ("javafx.concurrent.WorkerStateEvent", "WORKER_STATE_SCHEDULED"),
        FxEventType::ActionEvent_Action => ("javafx.event.ActionEvent", "ACTION"),
        FxEventType::ActionEvent_Any => ("javafx.event.ActionEvent", "ANY"),
        FxEventType::CheckboxTreeItem_TreeModificationEvent_Any => ("javafx.scene.control.CheckBoxTreeItem.TreeModificationEvent", "ANY"),
        FxEventType::DialogEvent_Any => ("javafx.scene.control.DialogEvent", "ANY"),
        FxEventType::DialogEvent_DialogCloseRequest => ("javafx.scene.control.DialogEvent", "DIALOG_CLOSE_REQUEST"),
        FxEventType::DialogEvent_DialogHidden => ("javafx.scene.control.DialogEvent", "DIALOG_HIDDEN"),
        FxEventType::DialogEvent_DialogHiding => ("javafx.scene.control.DialogEvent", "DIALOG_HIDING"),
        FxEventType::DialogEvent_DialogShowing => ("javafx.scene.control.DialogEvent", "DIALOG_SHOWING"),
        FxEventType::DialogEvent_DialogShown => ("javafx.scene.control.DialogEvent", "DIALOG_SHOWN"),
        FxEventType::Listview_EditEvent_Any => ("javafx.scene.control.ListView.EditEvent", "ANY"),
        FxEventType::ScrollToEvent_Any => ("javafx.scene.control.ScrollToEvent", "ANY"),
        FxEventType::SortEvent_Any => ("javafx.scene.control.SortEvent", "ANY"),
        FxEventType::TableColumn_CellEditEvent_Any => ("javafx.scene.control.TableColumn.CellEditEvent", "ANY"),
        FxEventType::TreeItem_TreeModificationEvent_Any => ("javafx.scene.control.TreeItem.TreeModificationEvent", "ANY"),
        FxEventType::TreeTableView_EditEvent_Any => ("javafx.scene.control.TreeTableView.EditEvent", "ANY"),
        FxEventType::TreeView_EditEvent_Any => ("javafx.scene.control.TreeView.EditEvent", "ANY"),
        FxEventType::ContextMenuEvent_Any => ("javafx.scene.input.ContextMenuEvent", "ANY"),
        FxEventType::ContextMenuEvent_ContextMenuRequested => ("javafx.scene.input.ContextMenuEvent", "CONTEXT_MENU_REQUESTED"),
        FxEventType::DragEvent_Any => ("javafx.scene.input.DragEvent", "ANY"),
        FxEventType::DragEvent_DragDone => ("javafx.scene.input.DragEvent", "DRAG_DONE"),
        FxEventType::DragEvent_DragDropped => ("javafx.scene.input.DragEvent", "DRAG_DROPPED"),
        FxEventType::DragEvent_DragEntered => ("javafx.scene.input.DragEvent", "DRAG_ENTERED"),
        FxEventType::DragEvent_DragEnteredTarget => ("javafx.scene.input.DragEvent", "DRAG_ENTERED_TARGET"),
        FxEventType::DragEvent_DragExited => ("javafx.scene.input.DragEvent", "DRAG_EXITED"),
        FxEventType::DragEvent_DragExitedTarget => ("javafx.scene.input.DragEvent", "DRAG_EXITED_TARGET"),
        FxEventType::DragEvent_DragOver => ("javafx.scene.input.DragEvent", "DRAG_OVER"),
        FxEventType::GestureEvent_Any => ("javafx.scene.input.GestureEvent", "ANY"),
        FxEventType::InputEvent_Any => ("javafx.scene.input.InputEvent", "ANY"),
        FxEventType::InputMethodEvent_Any => ("javafx.scene.input.InputMethodEvent", "ANY"),
        FxEventType::InputMethodEvent_InputMethodTextChanged => ("javafx.scene.input.InputMethodEvent", "INPUT_METHOD_TEXT_CHANGED"),
        FxEventType::KeyEvent_Any => ("javafx.scene.input.KeyEvent", "ANY"),
        FxEventType::KeyEvent_KeyPressed => ("javafx.scene.input.KeyEvent", "KEY_PRESSED"),
        FxEventType::KeyEvent_KeyReleased => ("javafx.scene.input.KeyEvent", "KEY_RELEASED"),
        FxEventType::KeyEvent_KeyTyped => ("javafx.scene.input.KeyEvent", "KEY_TYPED"),
        FxEventType::MouseDragEvent_Any => ("javafx.scene.input.MouseDragEvent", "ANY"),
        FxEventType::MouseDragEvent_DragDetected => ("javafx.scene.input.MouseDragEvent", "DRAG_DETECTED"),
        FxEventType::MouseDragEvent_MouseClicked => ("javafx.scene.input.MouseDragEvent", "MOUSE_CLICKED"),
        FxEventType::MouseDragEvent_MouseDragEntered => ("javafx.scene.input.MouseDragEvent", "MOUSE_DRAG_ENTERED"),
        FxEventType::MouseDragEvent_MouseDragEnteredTarget => ("javafx.scene.input.MouseDragEvent", "MOUSE_DRAG_ENTERED_TARGET"),
        FxEventType::MouseDragEvent_MouseDragExited => ("javafx.scene.input.MouseDragEvent", "MOUSE_DRAG_EXITED"),
        FxEventType::MouseDragEvent_MouseDragExitedTarget => ("javafx.scene.input.MouseDragEvent", "MOUSE_DRAG_EXITED_TARGET"),
        FxEventType::MouseDragEvent_MouseDragged => ("javafx.scene.input.MouseDragEvent", "MOUSE_DRAGGED"),
        FxEventType::MouseDragEvent_MouseDragOver => ("javafx.scene.input.MouseDragEvent", "MOUSE_DRAG_OVER"),
        FxEventType::MouseDragEvent_MouseDragReleased => ("javafx.scene.input.MouseDragEvent", "MOUSE_DRAG_RELEASED"),
        FxEventType::MouseDragEvent_MouseEntered => ("javafx.scene.input.MouseDragEvent", "MOUSE_ENTERED"),
        FxEventType::MouseDragEvent_MouseEnteredTarget => ("javafx.scene.input.MouseDragEvent", "MOUSE_ENTERED_TARGET"),
        FxEventType::MouseDragEvent_MouseExited => ("javafx.scene.input.MouseDragEvent", "MOUSE_EXITED"),
        FxEventType::MouseDragEvent_MouseExitedTarget => ("javafx.scene.input.MouseDragEvent", "MOUSE_EXITED_TARGET"),
        FxEventType::MouseDragEvent_MouseMoved => ("javafx.scene.input.MouseDragEvent", "MOUSE_MOVED"),
        FxEventType::MouseDragEvent_MousePressed => ("javafx.scene.input.MouseDragEvent", "MOUSE_PRESSED"),
        FxEventType::MouseDragEvent_MouseReleased => ("javafx.scene.input.MouseDragEvent", "MOUSE_RELEASED"),
        FxEventType::MouseEvent_Any => ("javafx.scene.input.MouseEvent", "ANY"),
        FxEventType::MouseEvent_DragDetected => ("javafx.scene.input.MouseEvent", "DRAG_DETECTED"),
        FxEventType::MouseEvent_MouseClicked => ("javafx.scene.input.MouseEvent", "MOUSE_CLICKED"),
        FxEventType::MouseEvent_MouseDragged => ("javafx.scene.input.MouseEvent", "MOUSE_DRAGGED"),
        FxEventType::MouseEvent_MouseEntered => ("javafx.scene.input.MouseEvent", "MOUSE_ENTERED"),
        FxEventType::MouseEvent_MouseEnteredTarget => ("javafx.scene.input.MouseEvent", "MOUSE_ENTERED_TARGET"),
        FxEventType::MouseEvent_MouseExited => ("javafx.scene.input.MouseEvent", "MOUSE_EXITED"),
        FxEventType::MouseEvent_MouseExitedTarget => ("javafx.scene.input.MouseEvent", "MOUSE_EXITED_TARGET"),
        FxEventType::MouseEvent_MouseMoved => ("javafx.scene.input.MouseEvent", "MOUSE_MOVED"),
        FxEventType::MouseEvent_MousePressed => ("javafx.scene.input.MouseEvent", "MOUSE_PRESSED"),
        FxEventType::MouseEvent_MouseReleased => ("javafx.scene.input.MouseEvent", "MOUSE_RELEASED"),
        FxEventType::RotateEvent_Any => ("javafx.scene.input.RotateEvent", "ANY"),
        FxEventType::RotateEvent_Rotate => ("javafx.scene.input.RotateEvent", "ROTATE"),
        FxEventType::RotateEvent_RotationFinished => ("javafx.scene.input.RotateEvent", "ROTATION_FINISHED"),
        FxEventType::RotateEvent_RotationStarted => ("javafx.scene.input.RotateEvent", "ROTATION_STARTED"),
        FxEventType::ScrollEvent_Any => ("javafx.scene.input.ScrollEvent", "ANY"),
        FxEventType::ScrollEvent_Scroll => ("javafx.scene.input.ScrollEvent", "SCROLL"),
        FxEventType::ScrollEvent_ScrollFinished => ("javafx.scene.input.ScrollEvent", "SCROLL_FINISHED"),
        FxEventType::ScrollEvent_ScrollStarted => ("javafx.scene.input.ScrollEvent", "SCROLL_STARTED"),
        FxEventType::SwipeEvent_Any => ("javafx.scene.input.SwipeEvent", "ANY"),
        FxEventType::SwipeEvent_SwipeDown => ("javafx.scene.input.SwipeEvent", "SWIPE_DOWN"),
        FxEventType::SwipeEvent_SwipeLeft => ("javafx.scene.input.SwipeEvent", "SWIPE_LEFT"),
        FxEventType::SwipeEvent_SwipeRight => ("javafx.scene.input.SwipeEvent", "SWIPE_RIGHT"),
        FxEventType::SwipeEvent_SwipeUp => ("javafx.scene.input.SwipeEvent", "SWIPE_UP"),
        FxEventType::TouchEvent_Any => ("javafx.scene.input.TouchEvent", "ANY"),
        FxEventType::TouchEvent_TouchMoved => ("javafx.scene.input.TouchEvent", "TOUCH_MOVED"),
        FxEventType::TouchEvent_TouchPressed => ("javafx.scene.input.TouchEvent", "TOUCH_PRESSED"),
        FxEventType::TouchEvent_TouchReleased => ("javafx.scene.input.TouchEvent", "TOUCH_RELEASED"),
        FxEventType::TouchEvent_TouchStationary => ("javafx.scene.input.TouchEvent", "TOUCH_STATIONARY"),
        FxEventType::ZoomEvent_Any => ("javafx.scene.input.ZoomEvent", "ANY"),
        FxEventType::ZoomEvent_Zoom => ("javafx.scene.input.ZoomEvent", "ZOOM"),
        FxEventType::ZoomEvent_ZoomFinished => ("javafx.scene.input.ZoomEvent", "ZOOM_FINISHED"),
        FxEventType::ZoomEvent_ZoomStarted => ("javafx.scene.input.ZoomEvent", "ZOOM_STARTED"),
        FxEventType::MediaMediaErrorEvent_Any => ("javafx.scene.media.MediaErrorEvent", "ANY"),
        FxEventType::MediaMediaErrorEvent_MediaError => ("javafx.scene.media.MediaErrorEvent", "MEDIA_ERROR"),
        FxEventType::MediaMediaMarkerEvent_Action => ("javafx.scene.media.MediaMarkerEvent", "ACTION"),
        FxEventType::MediaMediaMarkerEvent_Any => ("javafx.scene.media.MediaMarkerEvent", "ANY"),
        FxEventType::TransformChangedEvent_Any => ("javafx.scene.transform.TransformChangedEvent", "ANY"),
        FxEventType::TransformChangedEvent_TransformChanged => ("javafx.scene.transform.TransformChangedEvent", "TRANSFORM_CHANGED"),
        FxEventType::WindowEvent_Any => ("javafx.stage.WindowEvent", "ANY"),
        FxEventType::WindowEvent_WindowCloseRequest => ("javafx.stage.WindowEvent", "WINDOW_CLOSE_REQUEST"),
        FxEventType::WindowEvent_WindowHidden => ("javafx.stage.WindowEvent", "WINDOW_HIDDEN"),
        FxEventType::WindowEvent_WindowHiding => ("javafx.stage.WindowEvent", "WINDOW_HIDING"),
        FxEventType::WindowEvent_WindowShowing => ("javafx.stage.WindowEvent", "WINDOW_SHOWING"),
        FxEventType::WindowEvent_WindowShown => ("javafx.stage.WindowEvent", "WINDOW_SHOWN"),
    };
    (class.to_string(), field.to_string())
}

#[cfg(test)]
mod api_unit_tests {
    use crate::JvmBuilder;

    use super::*;

    #[test]
    #[should_panic]
    fn test_deploy_javafx_dependencies() {
        let jvm: Jvm = JvmBuilder::new().build().unwrap();
        jvm.deploy_javafx_dependencies().unwrap();
    }
}
