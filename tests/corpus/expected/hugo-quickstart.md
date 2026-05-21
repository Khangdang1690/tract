# Quick start

Create your first Hugo project.

In this tutorial you will:

1. Create a project
2. Add content
3. Configure the project
4. Publish the project

## Prerequisites

Before you begin this tutorial you must:

1. Install Hugo (any edition, v0.158.0 or later)
2. Install Git

You must also be comfortable working from the command line.

## Create a project

### Commands

**If you are a Windows user:**

- Do not use the Command Prompt
- Do not use Windows PowerShell
- Run these commands from PowerShell or a Linux terminal such as WSL or Git Bash

Verify that you have installed Hugo v0.158.0 or later.

```
hugo version
```

Run these commands to create a Hugo project with the Ananke theme.

```
hugo new project quickstart
cd quickstart
git init
git submodule add https://github.com/gohugo-ananke/ananke themes/ananke
echo "theme = 'ananke'" >> hugo.toml
hugo server
```

View your project at the URL displayed in your terminal. Press `Ctrl + C` to stop Hugo's development server.

### Explanation of commands

Create the project skeleton for your project in the `quickstart` directory.

Change the current directory to the root of your project.

Initialize an empty Git repository in the current directory.

Clone the Ananke theme into the `themes` directory, adding it to your project as a Git submodule.

Append a line to your project configuration file, indicating the current theme.

Start Hugo's development server.

Press `Ctrl + C` to stop Hugo's development server.

## Add content

Add a new page to your project.

```
hugo new content content/posts/my-first-post.md
```

Hugo created the file in the `content/posts` directory. Open the file with your editor.

Notice the `draft` value in the front matter is `true`. By default, Hugo does not publish draft content when you build the project.

Add some Markdown to the body of the post, but do not change the `draft` value.
