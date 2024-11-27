# Larium-rs ✨

Larium-rs - telegram bot for notification about new episode of anime.
Rewritten from [JS](https://github.com/Hkmori15/Larium)

# Settings 🔧:

1. Get token from FatherBot in Telegram
2. Create cluster in MongoDB Atlas and get URI Token
3. In root project create **.env** file and paste there's tokens

# Usage 🌌:

1. Command **/start** - guide how to use bot.
2. Command **/subscribe** - subscribe to anime for notification about new episode: `/subscribe "anime name"`
3. Command **/unsubscribe** - unsubscribe from anime notification: `/unsubscribe "anime name"`
4. Command **/list** - list of all anime that you subscribed.
5. Command **/info** - provide information about anime: `/info "anime name"`

# TODO-list 🎆:

- [x] Add command /info for providing information about anime.
- [x] Add autoremove anime from database when it's stop be ongoing.
- [x] Find better API for anime searching and info.
- [ ] Find a better way to check the release of an anime episodes (Cant find a way to fast check the release of an anime episode, idk how to do it).
