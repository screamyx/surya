//! Settings navigation and lazily loaded pages.

use super::*;

/// The settings sections (feature-inventory §1.5 routes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    Devices,
    /// Remote engines this app can drive over the network.
    Servers,
    /// Which harnesses the composer offers (enable/disable toggles).
    Harnesses,
    /// Per-provider CLI accounts (login, usage) — labeled "Accounts".
    Agents,
    ApprovalRules,
    Appearance,
    Notifications,
    Shortcuts,
    Archived,
}

impl SettingsSection {
    pub const ALL: [SettingsSection; 9] = [
        SettingsSection::Devices,
        SettingsSection::Servers,
        SettingsSection::Harnesses,
        SettingsSection::Agents,
        SettingsSection::ApprovalRules,
        SettingsSection::Appearance,
        SettingsSection::Notifications,
        SettingsSection::Shortcuts,
        SettingsSection::Archived,
    ];

    /// Sidebar + header label (surya settings-sidebar.tsx SECTIONS / __root.tsx
    /// `settingsTitle` — the same strings in both places).
    pub fn label(self) -> &'static str {
        match self {
            SettingsSection::Devices => "Devices",
            SettingsSection::Servers => "Servers",
            SettingsSection::Harnesses => "Agents",
            SettingsSection::Agents => "Accounts",
            SettingsSection::ApprovalRules => "Approval rules",
            SettingsSection::Appearance => "Appearance",
            SettingsSection::Notifications => "Notifications",
            SettingsSection::Shortcuts => "Shortcuts",
            SettingsSection::Archived => "Archived sessions",
        }
    }
}

impl Shell {
    pub(super) fn settings_outlet(
        &mut self,
        section: SettingsSection,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match section {
            SettingsSection::Devices => {
                if self.devices_page.is_none() {
                    let state = self.state.clone();
                    self.devices_page = Some(cx.new(|cx| DevicesPage::new(state, cx)));
                }
                match &self.devices_page {
                    Some(page) => page.clone().into_any_element(),
                    None => Empty.into_any_element(),
                }
            }
            SettingsSection::Servers => {
                if self.servers_page.is_none() {
                    let state = self.state.clone();
                    let servers = self.settings.servers.clone();
                    let active = self.settings.active_server.clone();
                    let page = cx.new(|cx| ServersPage::new(state, servers, active, cx));
                    self.servers_sub = Some(cx.subscribe(
                        &page,
                        |this: &mut Shell, _, event: &ServersEvent, cx| match event {
                            ServersEvent::Changed { servers, active } => {
                                this.settings.servers = servers.clone();
                                this.settings.active_server = active.clone();
                                this.schedule_save(cx);
                                cx.notify();
                            }
                            ServersEvent::Connect(target) => {
                                this.switch_engine(target.clone(), cx);
                            }
                        },
                    ));
                    self.servers_page = Some(page);
                }
                match &self.servers_page {
                    Some(page) => page.clone().into_any_element(),
                    None => Empty.into_any_element(),
                }
            }
            SettingsSection::Harnesses => {
                if self.harnesses_page.is_none() {
                    let state = self.state.clone();
                    self.harnesses_page = Some(cx.new(|cx| HarnessesPage::new(state, cx)));
                }
                match &self.harnesses_page {
                    Some(page) => page.clone().into_any_element(),
                    None => Empty.into_any_element(),
                }
            }
            SettingsSection::Agents => {
                if self.accounts_page.is_none() {
                    let state = self.state.clone();
                    self.accounts_page = Some(cx.new(|cx| AccountsPage::new(state, cx)));
                }
                match &self.accounts_page {
                    Some(page) => page.clone().into_any_element(),
                    None => Empty.into_any_element(),
                }
            }
            SettingsSection::Appearance => {
                if self.appearance_page.is_none() {
                    self.appearance_page = Some(cx.new(AppearancePage::new));
                }
                match &self.appearance_page {
                    Some(page) => page.clone().into_any_element(),
                    None => Empty.into_any_element(),
                }
            }
            SettingsSection::Notifications => {
                if self.notifications_page.is_none() {
                    let page = cx.new(|cx| {
                        NotificationsPage::new(
                            self.settings.sound_enabled,
                            self.settings.notifications_enabled,
                            self.settings.notifications_background_only,
                            cx,
                        )
                    });
                    // Persist the flags whenever the page flips one.
                    self.notifications_sub = Some(cx.subscribe(
                        &page,
                        |this: &mut Shell, _, event: &NotificationsEvent, cx| {
                            let NotificationsEvent::Changed {
                                sound,
                                desktop,
                                background_only,
                            } = *event;
                            this.settings.sound_enabled = sound;
                            this.settings.notifications_enabled = desktop;
                            this.settings.notifications_background_only = background_only;
                            this.schedule_save(cx);
                            cx.notify();
                        },
                    ));
                    self.notifications_page = Some(page);
                }
                match &self.notifications_page {
                    Some(page) => page.clone().into_any_element(),
                    None => Empty.into_any_element(),
                }
            }
            SettingsSection::Shortcuts => {
                if self.shortcuts_page.is_none() {
                    let state = self.state.clone();
                    let keymap = self.settings.keymap.clone();
                    let page = cx.new(|cx| ShortcutsPage::new(state, keymap, cx));
                    // Persist + re-apply the keymap whenever the page changes it.
                    self.shortcuts_sub = Some(cx.subscribe(
                        &page,
                        |this: &mut Shell, _, event: &ShortcutsEvent, cx| {
                            let ShortcutsEvent::Changed(keymap) = event;
                            this.settings.keymap = keymap.clone();
                            apply_keymap(cx, keymap);
                            this.schedule_save(cx);
                            cx.notify();
                        },
                    ));
                    self.shortcuts_page = Some(page);
                }
                match &self.shortcuts_page {
                    Some(page) => page.clone().into_any_element(),
                    None => Empty.into_any_element(),
                }
            }
            SettingsSection::ApprovalRules => {
                if self.approval_rules_page.is_none() {
                    let state = self.state.clone();
                    self.approval_rules_page = Some(cx.new(|cx| RulesPane::new(state, cx)));
                }
                self.approval_rules_page
                    .as_ref()
                    .unwrap()
                    .clone()
                    .into_any_element()
            }
            SettingsSection::Archived => {
                if self.archived_page.is_none() {
                    let state = self.state.clone();
                    self.archived_page = Some(cx.new(|cx| ArchivedPage::new(state, cx)));
                }
                match &self.archived_page {
                    Some(page) => page.clone().into_any_element(),
                    None => Empty.into_any_element(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_approval_rules_are_reachable_from_settings() {
        assert!(SettingsSection::ALL.contains(&SettingsSection::ApprovalRules));
        assert_eq!(SettingsSection::ApprovalRules.label(), "Approval rules");
        // The page headline and the sidebar row are the same words: the page
        // has no other title, so a drift here leaves the section unnamed.
        assert_eq!(
            SettingsSection::ApprovalRules.label(),
            RulesPane::PAGE_TITLE
        );
        let mut nav = NavHistory::new(NavEntry::Settings(SettingsSection::ApprovalRules));
        nav.push(NavEntry::Settings(SettingsSection::Devices));
        assert_eq!(
            nav.back(),
            Some(NavEntry::Settings(SettingsSection::ApprovalRules))
        );
    }
}
