module Pauan.Transaction (Transaction, TransactionId, runTransaction) where

import Prelude
import Control.Monad.Eff (Eff)


foreign import data Transaction :: # ! -> * -> *

foreign import data TransactionId :: *

foreign import runTransaction :: forall a eff. Transaction eff a -> Eff eff a


foreign import mapImpl :: forall a b eff. (a -> b) -> Transaction eff a -> Transaction eff b

instance functorTransaction :: Functor (Transaction eff) where
  map = mapImpl


foreign import applyImpl :: forall a b eff. Transaction eff (a -> b) -> Transaction eff a -> Transaction eff b

instance applyTransaction :: Apply (Transaction eff) where
  apply = applyImpl


foreign import bindImpl :: forall a b eff. Transaction eff a -> (a -> Transaction eff b) -> Transaction eff b

instance bindTransaction :: Bind (Transaction eff) where
  bind = bindImpl


foreign import pureImpl :: forall a eff. a -> Transaction eff a

instance applicativeTransaction :: Applicative (Transaction eff) where
  pure = pureImpl