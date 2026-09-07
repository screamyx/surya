use super::*;

impl Composer {
    /// Stage supported images and UTF-8 text from the picker, drop, or paste.
    pub(crate) fn add_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let mut staged = Vec::new();
        let mut failed = false;
        for path in &paths {
            match attachments::stage_file(path) {
                Ok(att) => staged.push(att),
                Err(message) => {
                    failed = true;
                    self.failure = Some(message.into());
                    self.failure_key = Some(self.current_key.clone());
                    cx.notify();
                }
            }
        }
        match batch_notice(staged.len(), 0, failed) {
            BatchNotice::Show(notice) => {
                self.failure = Some(notice.into());
                self.failure_key = Some(self.current_key.clone());
                cx.notify();
            }
            BatchNotice::Clear => {
                self.failure = None;
                self.failure_key = None;
                cx.notify();
            }
            BatchNotice::Leave => {}
        }
        self.add_staged(staged, cx);
    }

    /// The staged-thumbnail strip (attachment-ui.tsx AttachmentStrip):
    /// `flex flex-wrap gap-2 px-4 pt-3`, 56px rounded thumbs, a remove button
    /// revealed on hover, click opens the full-size preview.
    pub(super) fn render_attachment_strip(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Div> {
        let staged = self.staged();
        if staged.is_empty() {
            return None;
        }
        let mut strip = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(px(STRIP_GAP))
            .px(px(STRIP_PAD_X))
            .pt(px(STRIP_PAD_TOP));
        for (ix, att) in staged.iter().enumerate() {
            let group: SharedString = format!("composer-att-{}", att.id).into();
            let preview = att.image().map(|image| attachments::PreviewImage {
                name: att.name.clone().into(),
                image: image.clone(),
            });
            let remove_id = att.id.clone();
            strip = strip.child(
                div()
                    .group(group.clone())
                    .relative()
                    .child(
                        div()
                            .id(("composer-att-thumb", ix))
                            .size(px(STRIP_THUMB))
                            .rounded(px(8.0))
                            .overflow_hidden()
                            .border_1()
                            .border_color(crate::theme::hairline(0.10))
                            .when_some(preview, |el, preview| {
                                el.cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.preview = Some(preview.clone());
                                        this.preview_focus_pending = true;
                                        cx.notify();
                                    }))
                            })
                            .child(match att.image() {
                                Some(image) => img(image.clone())
                                    .w(px(STRIP_THUMB - 2.0))
                                    .h(px(STRIP_THUMB - 2.0))
                                    .rounded(px(7.0))
                                    .object_fit(ObjectFit::Cover)
                                    .into_any_element(),
                                None => attachments::file_tile(
                                    format!("composer-file-{}", att.id).into(),
                                    att.name.clone().into(),
                                    STRIP_THUMB - 2.0,
                                    STRIP_THUMB - 2.0,
                                    theme,
                                ),
                            }),
                    )
                    // Own layer: inside the frosted pill everything shares one
                    // draw order and images render last, so without it the
                    // thumbnail paints OVER this button (user report).
                    .child(crate::frost::layered(
                        div()
                            .id(("composer-att-remove", ix))
                            .absolute()
                            .top(px(-6.0))
                            .right(px(-6.0))
                            .size(px(18.0))
                            .rounded_full()
                            .bg(theme.bg)
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .shadow_sm()
                            .opacity(0.0)
                            .group_hover(group, |s| s.opacity(1.0))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                // The button overhangs the thumbnail, whose
                                // hitbox is right underneath — don't let the
                                // same click also open the preview.
                                cx.stop_propagation();
                                this.remove_attachment(&remove_id, cx);
                            }))
                            .child(
                                crate::icons::icon(crate::icons::CLOSE_CIRCLE)
                                    .size(px(14.0))
                                    .text_color(theme.text_muted),
                            ),
                    )),
            );
        }
        Some(strip)
    }
}
