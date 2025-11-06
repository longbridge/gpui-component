---
title: Form
description: Flexible form container with support for field layout, validation, and multi-column layouts.
---

# Form

A comprehensive form component that provides structured layout for form fields with support for vertical/horizontal layouts, validation, field groups, and responsive multi-column layouts.

## Import

```rust
use gpui_component::form::{form_field, v_form, h_form, Form, FormField};
```

## Usage

### Basic Form

```rust
v_form()
    .child(
        form_field()
            .label("Name")
            .child(Input::new(&name_input))
    )
    .child(
        form_field()
            .label("Email")
            .child(Input::new(&email_input))
            .required(true)
    )
```

### Horizontal Form Layout

```rust
h_form()
    .label_width(px(120.))
    .child(
        form_field()
            .label("First Name")
            .child(Input::new(&first_name))
    )
    .child(
        form_field()
            .label("Last Name")
            .child(Input::new(&last_name))
    )
```

### Multi-Column Form

```rust
v_form()
    .columns(2) // Two-column layout
    .child(
        form_field()
            .label("First Name")
            .child(Input::new(&first_name))
    )
    .child(
        form_field()
            .label("Last Name")
            .child(Input::new(&last_name))
    )
    .child(
        form_field()
            .label("Bio")
            .col_span(2) // Span across both columns
            .child(Input::new(&bio_input))
    )
```

## Form Container and Layout

### Vertical Layout (Default)

```rust
v_form()
    .gap(px(12.))
    .child(form_field().label("Name").child(input))
    .child(form_field().label("Email").child(email_input))
```

### Horizontal Layout

```rust
h_form()
    .label_width(px(100.))
    .child(form_field().label("Name").child(input))
    .child(form_field().label("Email").child(email_input))
```

### Custom Sizing

```rust
v_form()
    .large() // Large form size
    .label_text_size(rems(1.2))
    .child(form_field().label("Title").child(input))

v_form()
    .small() // Small form size
    .child(form_field().label("Code").child(input))
```

## Form Validation

### Required Fields

```rust
form_field()
    .label("Email")
    .required(true) // Shows asterisk (*) next to label
    .child(Input::new(&email_input))
```

### Field Descriptions

```rust
form_field()
    .label("Password")
    .description("Must be at least 8 characters long")
    .child(Input::new(&password_input))
```

### Dynamic Descriptions

```rust
form_field()
    .label("Bio")
    .description_fn(|_, _| {
        div().child("Use at most 100 words to describe yourself.")
    })
    .child(Input::new(&bio_input))
```

### Field Visibility

```rust
form_field()
    .label("Admin Settings")
    .visible(user.is_admin()) // Conditionally show field
    .child(Switch::new("admin-mode"))
```

## Submit Handling

### Basic Submit Pattern

```rust
struct FormView {
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
}

impl FormView {
    fn submit(&mut self, cx: &mut Context<Self>) {
        let name = self.name_input.read(cx).value();
        let email = self.email_input.read(cx).value();

        // Validate inputs
        if name.is_empty() || email.is_empty() {
            // Show validation error
            return;
        }

        // Submit form data
        self.handle_submit(name, email, cx);
    }
}

// Form with submit button
v_form()
    .child(form_field().label("Name").child(Input::new(&self.name_input)))
    .child(form_field().label("Email").child(Input::new(&self.email_input)))
    .child(
        form_field()
            .no_label_indent()
            .child(
                Button::new("submit")
                    .primary()
                    .child("Submit")
                    .on_click(cx.listener(|this, _, _, cx| this.submit(cx)))
            )
    )
```

### Form with Action Buttons

```rust
v_form()
    .child(form_field().label("Title").child(Input::new(&title)))
    .child(form_field().label("Content").child(Input::new(&content)))
    .child(
        form_field()
            .no_label_indent()
            .child(
                h_flex()
                    .gap_2()
                    .child(Button::new("save").primary().child("Save"))
                    .child(Button::new("cancel").child("Cancel"))
                    .child(Button::new("preview").outline().child("Preview"))
            )
    )
```

## Field Groups

### Related Fields

```rust
v_form()
    .child(
        form_field()
            .label("Name")
            .child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&first_name)))
                    .child(div().flex_1().child(Input::new(&last_name)))
            )
    )
    .child(
        form_field()
            .label("Address")
            .items_start() // Align to start for multi-line content
            .child(
                v_flex()
                    .gap_2()
                    .child(Input::new(&street))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().flex_1().child(Input::new(&city)))
                            .child(div().w(px(100.)).child(Input::new(&zip)))
                    )
            )
    )
```

### Custom Field Components

```rust
form_field()
    .label("Theme Color")
    .child(ColorPicker::new(&color_state).small())

form_field()
    .label("Birth Date")
    .description("We'll send you a birthday gift!")
    .child(DatePicker::new(&date_state))

form_field()
    .label("Notifications")
    .child(
        v_flex()
            .gap_2()
            .child(Switch::new("email").label("Email notifications"))
            .child(Switch::new("push").label("Push notifications"))
            .child(Switch::new("sms").label("SMS notifications"))
    )
```

### Conditional Fields

```rust
v_form()
    .child(
        form_field()
            .label("Account Type")
            .child(Select::new(&account_type))
    )
    .child(
        form_field()
            .label("Company Name")
            .visible(is_business_account) // Show only for business accounts
            .child(Input::new(&company_name))
    )
    .child(
        form_field()
            .label("Tax ID")
            .visible(is_business_account)
            .required(is_business_account)
            .child(Input::new(&tax_id))
    )
```

## Grid Layout and Positioning

### Column Spanning

```rust
v_form()
    .columns(3) // Three-column grid
    .child(form_field().label("First").child(input1))
    .child(form_field().label("Second").child(input2))
    .child(form_field().label("Third").child(input3))
    .child(
        form_field()
            .label("Full Width")
            .col_span(3) // Spans all three columns
            .child(Input::new(&full_width))
    )
```

### Column Positioning

```rust
v_form()
    .columns(4)
    .child(form_field().label("A").child(input_a))
    .child(form_field().label("B").child(input_b))
    .child(
        form_field()
            .label("Positioned")
            .col_start(1) // Start at column 1
            .col_span(2)  // Span 2 columns
            .child(input_positioned)
    )
```

### Responsive Layout

```rust
v_form()
    .columns(if is_mobile { 1 } else { 2 })
    .child(form_field().label("Name").child(name_input))
    .child(form_field().label("Email").child(email_input))
    .child(
        form_field()
            .label("Bio")
            .when(!is_mobile, |field| field.col_span(2))
            .child(bio_input)
    )
```

## Examples

### User Registration Form

```rust
struct RegistrationForm {
    first_name: Entity<InputState>,
    last_name: Entity<InputState>,
    email: Entity<InputState>,
    password: Entity<InputState>,
    confirm_password: Entity<InputState>,
    terms_accepted: bool,
}

impl Render for RegistrationForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_form()
            .large()
            .child(
                form_field()
                    .label("Personal Information")
                    .no_label_indent()
                    .child(
                        h_flex()
                            .gap_3()
                            .child(
                                div().flex_1().child(
                                    Input::new(&self.first_name)
                                        .placeholder("First name")
                                )
                            )
                            .child(
                                div().flex_1().child(
                                    Input::new(&self.last_name)
                                        .placeholder("Last name")
                                )
                            )
                    )
            )
            .child(
                form_field()
                    .label("Email")
                    .required(true)
                    .child(Input::new(&self.email))
            )
            .child(
                form_field()
                    .label("Password")
                    .required(true)
                    .description("Must be at least 8 characters")
                    .child(Input::new(&self.password))
            )
            .child(
                form_field()
                    .label("Confirm Password")
                    .required(true)
                    .child(Input::new(&self.confirm_password))
            )
            .child(
                form_field()
                    .no_label_indent()
                    .child(
                        Checkbox::new("terms")
                            .label("I agree to the Terms of Service")
                            .checked(self.terms_accepted)
                            .on_click(cx.listener(|this, checked, _, cx| {
                                this.terms_accepted = *checked;
                                cx.notify();
                            }))
                    )
            )
            .child(
                form_field()
                    .no_label_indent()
                    .child(
                        Button::new("register")
                            .primary()
                            .large()
                            .w_full()
                            .child("Create Account")
                    )
            )
    }
}
```

### Settings Form with Sections

```rust
v_form()
    .column(2)
    .child(
        form_field()
            .label("Profile")
            .no_label_indent()
            .col_span(2)
            .child(Divider::horizontal())
    )
    .child(
        form_field()
            .label("Display Name")
            .child(Input::new(&display_name))
    )
    .child(
        form_field()
            .label("Email")
            .child(Input::new(&email))
    )
    .child(
        form_field()
            .label("Bio")
            .col_span(2)
            .items_start()
            .child(Input::new(&bio))
    )
    .child(
        form_field()
            .label("Preferences")
            .no_label_indent()
            .col_span(2)
            .child(Divider::horizontal())
    )
    .child(
        form_field()
            .label("Theme")
            .child(Select::new(&theme_state))
    )
    .child(
        form_field()
            .label("Language")
            .child(Select::new(&language_state))
    )
    .child(
        form_field()
            .no_label_indent()
            .child(Switch::new("notifications").label("Enable notifications"))
    )
    .child(
        form_field()
            .no_label_indent()
            .child(Switch::new("marketing").label("Marketing emails"))
    )
```

### Contact Form

```rust
v_form()
    .child(
        form_field()
            .label("Contact Information")
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Select::new(&prefix_state)
                            .w(px(80.))
                    )
                    .child(
                        div().flex_1().child(
                            Input::new(&name_input)
                                .placeholder("Your name")
                        )
                    )
            )
    )
    .child(
        form_field()
            .label("Email")
            .required(true)
            .child(Input::new(&email_input))
    )
    .child(
        form_field()
            .label("Subject")
            .child(Select::new(&subject_state))
    )
    .child(
        form_field()
            .label("Message")
            .required(true)
            .items_start()
            .description("Please describe your inquiry in detail")
            .child(Input::new(&message_input))
    )
    .child(
        form_field()
            .no_label_indent()
            .child(
                h_flex()
                    .gap_2()
                    .justify_between()
                    .child(
                        Checkbox::new("copy")
                            .label("Send me a copy")
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("cancel").child("Cancel"))
                            .child(Button::new("send").primary().child("Send Message"))
                    )
            )
    )
```
