//! The person's choices about how the window behaves, as the browser takes them in.

use files_core::Behaviour;

use crate::browser::Browser;
use crate::effect::Effect;

type Stepped = (Browser, Vec<Effect>);

impl Browser {
    /// The same browser behaving as `behaviour` says, from the start (nothing is listed yet to
    /// arrange).
    pub fn with_behaviour(self, behaviour: Behaviour) -> Browser {
        Browser { behaviour, ..self }.refreshed()
    }

    /// The person changed a choice: the rows are arranged again under it.
    pub(crate) fn behave(self, behaviour: Behaviour) -> Stepped {
        (self.with_behaviour(behaviour), Vec::new())
    }
}
