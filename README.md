# bach

Under construction. I am now looking for crontributors, and I will add documentation soon (I'm quite busy on proprietary software dev for now but I want to gather a community of devs willing to design and improve a way to standardize the way we do backups).

So for now, If you think it's useful and want to contribute, I will need hands and brains to do things, such as making it more reliable, write new modules that can handle different backup paradigms (for now I've only wrote rsync support, as needed by my former company, but I've got ideas on how to use libvirt to do differencial backups on virtual machines, and some other stuff).

The way it works is quite simple, It dynamically loads modules and connects them to a common message wire (every single module is some kind of daemon). But It needs explanations for someone coming fresh on the project. So if you want to contribute, just send me a message and let's have a chat. Your contribution is more than welcomed (Of course, I will add docs and contributing guidelines later).
