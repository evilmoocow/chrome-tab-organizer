module Pauan.Chrome.Windows where

import Prelude
import Control.Monad.Eff (Eff)
import Control.Monad.Eff.Exception (Error)
import Control.Monad.Aff (Aff, makeAff)
import Data.Maybe (Maybe(..))
import Data.Nullable (Nullable, toNullable)
import Data.Function.Uncurried (Fn10, runFn10, Fn9, runFn9)
import Pauan.Prelude (sleep, TIMER)


foreign import data WindowsState :: *

foreign import data Window :: *

foreign import data Tab :: *


foreign import initializeImpl :: forall e.
  Unit ->
  (((Error -> Eff e Unit) -> (WindowsState -> Eff e Unit) -> Eff e Unit) -> Aff e WindowsState) ->
  Aff e WindowsState

initialize :: forall e. Aff e WindowsState
initialize = initializeImpl unit makeAff


foreign import windows :: forall e. WindowsState -> Eff e (Array Window)

foreign import windowId :: Window -> Int

foreign import windowIsIncognito :: Window -> Boolean

foreign import windowIsFocused :: forall e. Window -> Eff e Boolean

foreign import windowTabs :: forall e. Window -> Eff e (Array Tab)


type Coordinates =
  { left :: Int
  , top :: Int
  , width :: Int
  , height :: Int }

data WindowState
  = Regular Coordinates
  | Docked Coordinates
  | Minimized
  | Maximized
  | Fullscreen

windowStateToString :: WindowState -> String
windowStateToString (Regular _) = "normal"
windowStateToString (Docked _) = "docked"
windowStateToString Minimized = "minimized"
windowStateToString Maximized = "maximized"
windowStateToString Fullscreen = "fullscreen"

windowStateToCoordinates :: WindowState -> Maybe Coordinates
windowStateToCoordinates (Regular a) = Just a
windowStateToCoordinates (Docked a) = Just a
windowStateToCoordinates _ = Nothing


foreign import windowStateImpl :: forall e.
  Unit ->
  (((Error -> Eff e Unit) -> (WindowState -> Eff e Unit) -> Eff e Unit) -> Aff e WindowState) ->
  (Int -> Int -> Int -> Int -> WindowState) ->
  (Int -> Int -> Int -> Int -> WindowState) ->
  WindowState ->
  WindowState ->
  WindowState ->
  Window ->
  Aff e WindowState

windowState :: forall e. Window -> Aff e WindowState
windowState = windowStateImpl unit makeAff
  (\left top width height -> Regular { left, top, width, height })
  (\left top width height -> Docked { left, top, width, height })
  Minimized
  Maximized
  Fullscreen


foreign import windowCoordinatesImpl :: forall e.
  Unit ->
  (((Error -> Eff e Unit) -> (Coordinates -> Eff e Unit) -> Eff e Unit) -> Aff e Coordinates) ->
  (Int -> Int -> Int -> Int -> Coordinates) ->
  Window ->
  Aff e Coordinates

windowCoordinates :: forall e. Window -> Aff e Coordinates
windowCoordinates = windowCoordinatesImpl unit makeAff { left: _, top: _, width: _, height: _ }


foreign import closeWindowImpl :: forall e.
  Unit ->
  (((Error -> Eff e Unit) -> (Unit -> Eff e Unit) -> Eff e Unit) -> Aff e Unit) ->
  Window ->
  Aff e Unit

closeWindow :: forall e. Window -> Aff e Unit
closeWindow = closeWindowImpl unit makeAff


foreign import makeNewWindowImpl :: forall e.
  Unit ->
  (((Error -> Eff e Unit) -> (Unit -> Eff e Unit) -> Eff e Unit) -> Aff e Unit) ->
  String ->
  String ->
  Nullable Int ->
  Nullable Int ->
  Nullable Int ->
  Nullable Int ->
  Boolean ->
  Boolean ->
  Array String ->
  WindowsState ->
  Aff e Window

makeNewWindow :: forall e.
  WindowsState ->
  { type :: WindowType
  , state :: WindowState
  , focused :: Boolean
  , incognito :: Boolean
  , tabs :: Array String } ->
  Aff e Window
makeNewWindow state info =
  makeNewWindowImpl
    unit
    makeAff
    (windowTypeToString info.type)
    (windowStateToString info.state)
    -- TODO make this faster
    (toNullable $ map _.left coords)
    (toNullable $ map _.top coords)
    (toNullable $ map _.width coords)
    (toNullable $ map _.height coords)
    info.focused
    info.incognito
    info.tabs
    state
  where
    coords = windowStateToCoordinates info.state


foreign import changeWindowImpl :: forall e. Fn10
  Unit
  (((Error -> Eff e Unit) -> (Unit -> Eff e Unit) -> Eff e Unit) -> Aff e Unit)
  (Nullable String)
  (Nullable Int)
  (Nullable Int)
  (Nullable Int)
  (Nullable Int)
  (Nullable Boolean)
  (Nullable Boolean)
  Window
  (Aff e Unit)

changeWindow :: forall e.
  { state :: Maybe WindowState
  , focused :: Maybe Boolean
  , drawAttention :: Maybe Boolean } ->
  Window ->
  Aff e Unit
changeWindow info window =
  runFn10 changeWindowImpl
    unit
    makeAff
    (toNullable $ map windowStateToString info.state)
    -- TODO make this faster
    (toNullable $ map _.left coords)
    (toNullable $ map _.top coords)
    (toNullable $ map _.width coords)
    (toNullable $ map _.height coords)
    (toNullable info.focused)
    (toNullable info.drawAttention)
    window
  where
    -- TODO make this faster
    coords = bind info.state windowStateToCoordinates


data WindowType = Normal | Popup

windowTypeToString :: WindowType -> String
windowTypeToString Normal = "normal"
windowTypeToString Popup = "popup"

foreign import windowTypeImpl :: WindowType -> WindowType -> Window -> WindowType

windowType :: Window -> WindowType
windowType = windowTypeImpl Normal Popup


windowIsNormal :: Window -> Boolean
windowIsNormal window =
  case windowType window of
    Normal -> true
    _ -> false


windowIsPopup :: Window -> Boolean
windowIsPopup window =
  case windowType window of
    Popup -> true
    _ -> false


getMaximizedWindowCoordinates :: forall e. WindowsState -> Aff (timer :: TIMER | e) Coordinates
getMaximizedWindowCoordinates state = do
  win <- makeNewWindow state
    { type: Normal
    , state: Maximized
    , focused: true
    , incognito: false
    , tabs: [] }
  -- TODO this probably isn't needed, but it's better safe than sorry
  sleep 500
  coords <- windowCoordinates win
  closeWindow win
  pure coords