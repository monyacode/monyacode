use crate::commit_view::CommitView;
use git::blame::BlameEntry;
use git::commit::ParsedCommitMessage;
use git::repository::CommitSummary;
use gpui::{
    Element, Entity, MouseButton, ParentElement, Render, ScrollHandle, StatefulInteractiveElement, WeakEntity,
    prelude::*,
};
use markdown::{Markdown, MarkdownElement, style::MarkdownStyle};
use project::git_store::Repository;
use settings::Settings;
use theme::ThemeSettings;
use time::{OffsetDateTime, UtcOffset};
use ui::{CopyButton, Divider, prelude::*, tooltip_container};
use workspace::Workspace;

#[derive(Clone, Debug)]
pub struct CommitDetails {
    pub sha: SharedString,
    pub author_name: SharedString,
    pub author_email: SharedString,
    pub commit_time: OffsetDateTime,
    pub message: Option<ParsedCommitMessage>,
}

pub struct CommitAvatar {
    size: Option<IconSize>,
}

impl CommitAvatar {
    pub fn new() -> Self {
        Self { size: None }
    }

    pub fn from_commit_details() -> Self {
        Self { size: None }
    }

    pub fn size(mut self, size: IconSize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn render(&self) -> AnyElement {
        Icon::new(IconName::Person)
            .color(Color::Muted)
            .when_some(self.size, |this, size| this.size(size))
            .into_any_element()
    }
}

pub struct CommitTooltip {
    commit: CommitDetails,
    scroll_handle: ScrollHandle,
    markdown: Entity<Markdown>,
    repository: Entity<Repository>,
    workspace: WeakEntity<Workspace>,
}

impl CommitTooltip {
    pub fn blame_entry(
        blame: &BlameEntry,
        details: Option<ParsedCommitMessage>,
        repository: Entity<Repository>,
        workspace: WeakEntity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        let commit_time = blame
            .committer_time
            .and_then(|t| OffsetDateTime::from_unix_timestamp(t).ok())
            .unwrap_or(OffsetDateTime::now_utc());

        Self::new(
            CommitDetails {
                sha: blame.sha.to_string().into(),
                commit_time,
                author_name: blame.author.clone().unwrap_or("<no name>".to_string()).into(),
                author_email: blame.author_mail.clone().unwrap_or("".to_string()).into(),
                message: details,
            },
            repository,
            workspace,
            cx,
        )
    }

    pub fn new(
        commit: CommitDetails,
        repository: Entity<Repository>,
        workspace: WeakEntity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        let markdown = cx.new(|cx| {
            Markdown::new(
                commit
                    .message
                    .as_ref()
                    .map(|message| message.message.clone())
                    .unwrap_or_default(),
                None,
                None,
                cx,
            )
        });
        Self {
            commit,
            repository,
            workspace,
            scroll_handle: ScrollHandle::new(),
            markdown,
        }
    }
}

impl Render for CommitTooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let avatar = CommitAvatar::new().render();

        let author = self.commit.author_name.clone();

        let author_email = self.commit.author_email.clone();

        let short_commit_id = self
            .commit
            .sha
            .get(0..8)
            .map(|sha| sha.to_string().into())
            .unwrap_or_else(|| self.commit.sha.clone());
        let full_sha = self.commit.sha.to_string();
        let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        let absolute_timestamp = time_format::format_localized_timestamp(
            self.commit.commit_time,
            OffsetDateTime::now_utc(),
            local_offset,
            time_format::TimestampFormat::MediumAbsolute,
        );
        let markdown_style = MarkdownStyle::hover(window, cx);

        let message = self
            .commit
            .message
            .as_ref()
            .map(|_| MarkdownElement::new(self.markdown.clone(), markdown_style).into_any())
            .unwrap_or("<no commit message>".into_any());

        let pull_request = self
            .commit
            .message
            .as_ref()
            .and_then(|details| details.pull_request.clone());

        let ui_font_size = ThemeSettings::get_global(cx).ui_font_size(cx);
        let message_max_height = window.line_height() * 12 + (ui_font_size / 0.4);
        let repo = self.repository.clone();
        let workspace = self.workspace.clone();
        let commit_summary = CommitSummary {
            sha: self.commit.sha.clone(),
            subject: self.commit.message.as_ref().map_or(Default::default(), |message| {
                message
                    .message
                    .split('\n')
                    .next()
                    .unwrap()
                    .trim_end()
                    .to_string()
                    .into()
            }),
            commit_timestamp: self.commit.commit_time.unix_timestamp(),
            author_name: self.commit.author_name.clone(),
            has_parent: false,
        };

        tooltip_container(cx, move |this, cx| {
            this.occlude()
                .on_mouse_move(|_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    v_flex()
                        .w(gpui::rems(30.))
                        .gap_4()
                        .child(
                            h_flex()
                                .pb_1p5()
                                .gap_x_2()
                                .overflow_x_hidden()
                                .flex_wrap()
                                .child(avatar)
                                .child(author)
                                .when(!author_email.is_empty(), |this| {
                                    this.child(div().text_color(cx.theme().colors().text_muted).child(author_email))
                                })
                                .border_b_1()
                                .border_color(cx.theme().colors().border_variant),
                        )
                        .child(
                            div()
                                .id("inline-blame-commit-message")
                                .child(message)
                                .max_h(message_max_height)
                                .overflow_y_scroll()
                                .track_scroll(&self.scroll_handle),
                        )
                        .child(
                            h_flex()
                                .text_color(cx.theme().colors().text_muted)
                                .w_full()
                                .justify_between()
                                .pt_1p5()
                                .border_t_1()
                                .border_color(cx.theme().colors().border_variant)
                                .child(absolute_timestamp)
                                .child(
                                    h_flex()
                                        .gap_1p5()
                                        .when_some(pull_request, |this, pr| {
                                            this.child(
                                                Button::new("pull-request-button", format!("#{}", pr.number))
                                                    .color(Color::Muted)
                                                    .icon(IconName::PullRequest)
                                                    .icon_color(Color::Muted)
                                                    .icon_position(IconPosition::Start)
                                                    .style(ButtonStyle::Subtle)
                                                    .on_click(move |_, _, cx| {
                                                        cx.stop_propagation();
                                                        cx.open_url(pr.url.as_str())
                                                    }),
                                            )
                                            .child(Divider::vertical())
                                        })
                                        .child(
                                            Button::new("commit-sha-button", short_commit_id.clone())
                                                .style(ButtonStyle::Subtle)
                                                .color(Color::Muted)
                                                .icon(IconName::FileGit)
                                                .icon_color(Color::Muted)
                                                .icon_position(IconPosition::Start)
                                                .on_click(move |_, window, cx| {
                                                    CommitView::open(
                                                        commit_summary.sha.to_string(),
                                                        repo.downgrade(),
                                                        workspace.clone(),
                                                        None,
                                                        None,
                                                        crate::commit_view::OpenMode::Full,
                                                        window,
                                                        cx,
                                                    );
                                                    cx.stop_propagation();
                                                }),
                                        )
                                        .child(Divider::vertical())
                                        .child(CopyButton::new("copy-commit-sha", full_sha).tooltip_label("Copy SHA")),
                                ),
                        ),
                )
        })
    }
}

fn blame_entry_timestamp(blame_entry: &BlameEntry, format: time_format::TimestampFormat) -> String {
    match blame_entry.author_offset_date_time() {
        Ok(timestamp) => {
            let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
            time_format::format_localized_timestamp(timestamp, time::OffsetDateTime::now_utc(), local_offset, format)
        }
        Err(_) => "Error parsing date".to_string(),
    }
}

pub fn blame_entry_relative_timestamp(blame_entry: &BlameEntry) -> String {
    blame_entry_timestamp(blame_entry, time_format::TimestampFormat::Relative)
}
