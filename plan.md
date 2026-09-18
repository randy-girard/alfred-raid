We are building the start of a raid assistant app for the Project1999 Guild, GoodGuys. Alfred is our bot persona that helps us managing things. This app will have the following features:

- It will be cross platform compile-able. We want windows, mac, linux versions. It should be able to be opened by running a single executable if possible. Pick a language that will satisfy all of this and be performant. It also has to have a clean UI interface for doing the types of things we want to do
- It should have a tray icon that allows us to show the window and exit it.
- There will be a settings page where we will add our EverQuest directory via a browse function.
- The app will monotor the Logs folder of the EverQuest directory for log files and activate the one that is currently receiving updates from the game. It should also be able to seamlessly switch log files, regardless of the OS its on and also if the directory is a symlink and that symlink updates. We would really like to receive events if we can, and would rather not POLL the file, but if we cant get updates from the file via OS events, then we will have to poll at a decently good rate. We should also start at the end of the file and tail it as new content comes in.
- It should store configuration in a local .ini file, stored at a location based on the OS type, since they do them different.
- The first panel we will have, that can be activated via the tray is the cleric chain one. Its details will be below:

- It will monitor the logs for a syntax. This syntax to parse this should be in the ini file. Its like like:
    - "Player shouts, "GG 001 CH -- Target"
    - "You shout, "GG 001 CH -- Target"
    - Try to find variations of the "GG CH 001" and "CC 001 CH" etc.
    - It should pull out who shouted and the target, as well as the chain number.
    - If a cleric puts a ch macro up that we can kind of detect but is wrong, display a message on the panel as a warning.

- it should also parse guild chat, 'Someone tells the guild, "<message>"' for the following commands:
    - !mt <tank> - sets the current tank
    - !skip 001 - skips this cleric in the chain
    - !back 001 - returns to the chain
    - !reset-chain - clears the chains
    - !take 001 - sets a cleric to a specific number in the chain. We should detect which player is YOU by "You tell the guild". 
    - !move 001 002 - moves the cleric at 001 to 002. Thesen umbers have to be set.
    - !chain 2 - set the chain to 2 seconds.

- Have a panel that shows each cleric in the chain along with a progress bar that counts down and who is currently on the chain and then who is next. When its about to be you, the panel can show an extra indication that you are next in X seconds. Also have an optional sound that can play.